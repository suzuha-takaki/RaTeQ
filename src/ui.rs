use crate::{
    cli::Options,
    files::{self, FileType, Files},
    graphics,
    persistence::Store,
    render,
};
use image::DynamicImage;
use ratatui::{
    crossterm::{
        event::{
            self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent,
            KeyEventKind, KeyModifiers,
        },
        execute,
    },
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use ratatui_image::{picker::Picker, Resize, StatefulImage};
use std::{
    io::{self, Read, Write},
    process::{Child, Command, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

struct Job {
    child: Child,
    out: JoinHandle<io::Result<Vec<u8>>>,
    err: JoinHandle<io::Result<Vec<u8>>>,
    source: String,
    width: Option<u32>,
}
impl Job {
    fn start(source: String, width: Option<u32>) -> io::Result<Self> {
        let mut child = Command::new(std::env::current_exe()?)
            .arg("--internal-render")
            .arg(width.unwrap_or(0).to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut input = child.stdin.take().unwrap();
        let copy = source.clone();
        thread::spawn(move || {
            let _ = input.write_all(copy.as_bytes());
        });
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let out = thread::spawn(move || {
            let mut b = Vec::new();
            stdout.read_to_end(&mut b)?;
            Ok(b)
        });
        let err = thread::spawn(move || {
            let mut b = Vec::new();
            stderr.read_to_end(&mut b)?;
            Ok(b)
        });
        Ok(Self {
            child,
            out,
            err,
            source,
            width,
        })
    }
    fn cancel(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = self.out.join();
        let _ = self.err.join();
    }
}
#[derive(PartialEq)]
enum Mode {
    Edit,
    Find,
    Exit,
    Export,
    File,
}
struct Help {
    scroll: u16,
    page_height: u16,
}
struct FileEntry {
    kind: FileType,
    text: Vec<char>,
    cursor: usize,
    from_exit: bool,
    candidates: Option<Vec<String>>,
    candidate_scroll: u16,
    completed: bool,
    error: Option<String>,
}
impl FileEntry {
    fn path(&self) -> String {
        self.text.iter().collect()
    }
    fn edited(&mut self) {
        self.candidates = None;
        self.candidate_scroll = 0;
        self.completed = false;
        self.error = None;
    }
}
struct App {
    opts: Options,
    chars: Vec<char>,
    cursor: usize,
    initial: String,
    edit_scroll: (u16, u16),
    preview: Option<DynamicImage>,
    retained: Option<String>,
    render_width: Option<u32>,
    job: Option<Job>,
    reflow_job: Option<Job>,
    failed_reflow_width: Option<u32>,
    deadline: Option<Instant>,
    failed: bool,
    diagnostic: String,
    mode: Mode,
    help: Option<Help>,
    files: Files,
    persistence: Option<Store>,
    exit_warning: Option<String>,
    file_entry: Option<FileEntry>,
    search: String,
    message: String,
    single_preview: bool,
    x: u32,
    y: u32,
}
impl App {
    fn new(opts: Options, source: String) -> Self {
        let files = Files::new(
            std::env::current_dir().unwrap_or_default(),
            opts.file.clone(),
        );
        Self {
            opts,
            chars: source.chars().collect(),
            cursor: 0,
            initial: source,
            edit_scroll: (0, 0),
            preview: None,
            retained: None,
            render_width: None,
            job: None,
            reflow_job: None,
            failed_reflow_width: None,
            deadline: None,
            failed: false,
            diagnostic: String::new(),
            mode: Mode::Edit,
            help: None,
            files,
            persistence: None,
            exit_warning: None,
            file_entry: None,
            search: String::new(),
            message: String::new(),
            single_preview: false,
            x: 0,
            y: 0,
        }
    }
    fn source(&self) -> String {
        self.chars.iter().collect()
    }
    fn changed(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel();
        }
        self.failed = false;
        self.diagnostic.clear();
        self.message.clear();
        self.deadline = self
            .opts
            .live
            .then(|| Instant::now() + Duration::from_millis(300));
    }
    fn start(&mut self, width: u32) {
        if let Some(job) = self.job.take() {
            job.cancel();
        }
        if let Some(job) = self.reflow_job.take() {
            job.cancel();
        }
        self.deadline = None;
        self.failed = false;
        self.diagnostic.clear();
        match Job::start(self.source(), self.opts.breaking.then_some(width)) {
            Ok(job) => self.job = Some(job),
            Err(e) => {
                self.failed = true;
                self.diagnostic = format!("Renderer could not start: {e}");
            }
        }
    }
    fn finish(&mut self) {
        let done = match self.job.as_mut().map(|j| j.child.try_wait()) {
            Some(Ok(Some(status))) => Some(status),
            Some(Err(e)) => {
                self.failed = true;
                self.diagnostic = e.to_string();
                if let Some(job) = self.job.take() {
                    job.cancel();
                }
                None
            }
            _ => None,
        };
        if let Some(status) = done {
            let job = self.job.take().unwrap();
            let out = job.out.join().ok().and_then(Result::ok).unwrap_or_default();
            let err = job.err.join().ok().and_then(Result::ok).unwrap_or_default();
            if status.success() {
                match render::white_image(&out) {
                    Ok(image) => {
                        if let Some(job) = self.reflow_job.take() {
                            job.cancel();
                        }
                        self.preview = Some(image);
                        self.retained = Some(job.source);
                        self.render_width = job.width;
                        self.failed_reflow_width = None;
                        self.x = 0;
                        self.y = 0;
                        self.failed = false;
                    }
                    Err(e) => {
                        self.failed = true;
                        self.diagnostic = e;
                    }
                }
            } else {
                self.failed = true;
                self.diagnostic = String::from_utf8_lossy(&err).trim().to_owned();
            }
        }
    }

    fn apply_reflow(&mut self, source: &str, width: u32, current_width: u32, image: DynamicImage) {
        if width == current_width && self.retained.as_deref() == Some(source) {
            self.preview = Some(image);
            self.render_width = Some(width);
            self.failed_reflow_width = None;
        }
    }

    fn finish_reflow(&mut self, current_width: u32) {
        let done = match self.reflow_job.as_mut().map(|job| job.child.try_wait()) {
            Some(Ok(Some(status))) => Some(status),
            Some(Err(_)) => {
                if let Some(job) = self.reflow_job.take() {
                    self.failed_reflow_width = job.width;
                    job.cancel();
                }
                None
            }
            _ => None,
        };
        if let Some(status) = done {
            let job = self.reflow_job.take().unwrap();
            let output = job.out.join().ok().and_then(Result::ok).unwrap_or_default();
            let _ = job.err.join();
            if let Some(width) = job.width {
                if status.success() {
                    match render::white_image(&output) {
                        Ok(image) => self.apply_reflow(&job.source, width, current_width, image),
                        Err(_) => self.failed_reflow_width = Some(width),
                    }
                } else {
                    self.failed_reflow_width = Some(width);
                }
            }
        }
    }

    fn schedule_reflow(&mut self, width: u32) {
        if !self.opts.breaking || self.job.is_some() || self.deadline.is_some() {
            return;
        }
        if self
            .reflow_job
            .as_ref()
            .is_some_and(|job| job.width != Some(width))
        {
            self.reflow_job.take().unwrap().cancel();
        }
        let Some(source) = self.retained.as_ref() else {
            return;
        };
        if self.render_width == Some(width)
            || self.failed_reflow_width == Some(width)
            || self.reflow_job.is_some()
        {
            return;
        }
        match Job::start(source.clone(), Some(width)) {
            Ok(job) => self.reflow_job = Some(job),
            Err(_) => self.failed_reflow_width = Some(width),
        }
    }
    fn status(&self) -> &'static str {
        if self.job.is_some() {
            "Rendering"
        } else if self.failed {
            "Failed"
        } else if self.retained.as_ref().is_some_and(|s| *s == self.source()) {
            "Current"
        } else if self.retained.is_some() {
            "Outdated"
        } else {
            "Awaiting render"
        }
    }
    fn insert(&mut self, text: &str) {
        let n = text.chars().count();
        self.chars.splice(self.cursor..self.cursor, text.chars());
        self.cursor += n;
        if n > 0 {
            self.changed();
        }
    }
    fn line_col(&self) -> (usize, usize) {
        let before = &self.chars[..self.cursor];
        let row = before.iter().filter(|&&c| c == '\n').count();
        let start = before.iter().rposition(|&c| c == '\n').map_or(0, |i| i + 1);
        (row, self.cursor - start)
    }
    fn vertical(&mut self, down: bool) {
        let (row, col) = self.line_col();
        let starts: Vec<usize> = std::iter::once(0)
            .chain(
                self.chars
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &c)| (c == '\n').then_some(i + 1)),
            )
            .collect();
        let target = if down {
            row.checked_add(1).filter(|&i| i < starts.len())
        } else {
            row.checked_sub(1)
        };
        if let Some(t) = target {
            let end = self.chars[starts[t]..]
                .iter()
                .position(|&c| c == '\n')
                .map_or(self.chars.len(), |i| starts[t] + i);
            self.cursor = (starts[t] + col).min(end);
        }
    }
    fn find_next(&mut self) {
        if self.search.is_empty() {
            self.message.clear();
            return;
        }
        let needle: Vec<char> = self.search.chars().collect();
        let matches: Vec<usize> = self
            .chars
            .windows(needle.len())
            .enumerate()
            .filter_map(|(i, w)| (w == needle).then_some(i))
            .collect();
        if matches.is_empty() {
            self.message = "No matches".into();
            return;
        }
        self.cursor = matches
            .iter()
            .find(|&&i| i > self.cursor)
            .copied()
            .unwrap_or(matches[0]);
        self.message.clear();
    }

    fn open_file(&mut self, kind: FileType, from_exit: bool) {
        let text: Vec<char> = self.files.default_path(kind).chars().collect();
        self.file_entry = Some(FileEntry {
            kind,
            cursor: text.len(),
            text,
            from_exit,
            candidates: None,
            candidate_scroll: 0,
            completed: false,
            error: None,
        });
        self.message.clear();
        self.mode = Mode::File;
    }

    fn export_warning(&self) -> Option<&'static str> {
        if self.retained.is_none() {
            Some("E301 ERROR: Render a preview before exporting")
        } else if self.failed || self.retained.as_ref().is_some_and(|s| *s != self.source()) {
            Some("W205 WARNING: Exporting the displayed preview, which differs from the current source or predates a failed render")
        } else {
            None
        }
    }

    // Returns true only for a successful save requested by Exit.
    fn submit_file(&mut self) -> bool {
        let entry = self.file_entry.as_ref().unwrap();
        let kind = entry.kind;
        let from_exit = entry.from_exit;
        let path = match self.files.validate(&entry.path(), kind) {
            Ok((path, _)) => path,
            Err(_) => return false,
        };
        let contents = match kind {
            FileType::Source => Ok(self.source().into_bytes()),
            FileType::Png => self
                .retained
                .as_deref()
                .ok_or("E301 ERROR: Render a preview before exporting".to_string())
                .and_then(render::export_png),
            FileType::Svg => self
                .retained
                .as_deref()
                .ok_or("E301 ERROR: Render a preview before exporting".to_string())
                .and_then(render::export_svg),
        };
        let result = contents.and_then(|bytes| {
            std::fs::write(&path, bytes).map_err(|e| format!("E205 ERROR: Cannot write file: {e}"))
        });
        if let Err(e) = result {
            self.file_entry.as_mut().unwrap().error = Some(e);
            return false;
        }
        if let Err(e) = self.files.saved(kind, &path) {
            self.file_entry.as_mut().unwrap().error =
                Some(format!("E205 ERROR: Cannot record saved path: {e}"));
            return false;
        }
        let history_warning = if kind == FileType::Source {
            self.initial = self.source();
            self.persistence
                .as_ref()
                .and_then(|store| store.record_source(&path).err())
                .map(|e| {
                    format!(
                        "W206 WARNING: Source saved, but resume history could not be updated: {e}"
                    )
                })
        } else {
            None
        };
        self.file_entry = None;
        self.mode = Mode::Edit;
        self.message = history_warning
            .clone()
            .unwrap_or_else(|| format!("Wrote {}", path.display()));
        if from_exit {
            self.exit_warning = history_warning;
        }
        from_exit
    }
}
fn key(app: &mut App, key: KeyEvent, width: u32) -> bool {
    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
        return false;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if let Some(help) = app.help.as_mut() {
        match key.code {
            KeyCode::Char('x') if ctrl => app.help = None,
            KeyCode::Up => help.scroll = help.scroll.saturating_sub(1),
            KeyCode::Down => help.scroll = help.scroll.saturating_add(1),
            KeyCode::PageUp => help.scroll = help.scroll.saturating_sub(help.page_height.max(1)),
            KeyCode::PageDown => help.scroll = help.scroll.saturating_add(help.page_height.max(1)),
            _ => {}
        }
        return false;
    }
    if key.code == KeyCode::Char('g') && ctrl {
        app.help = Some(Help {
            scroll: 0,
            page_height: 1,
        });
        return false;
    }
    if app.mode == Mode::File {
        if key.code == KeyCode::Char('x') && ctrl {
            let from_exit = app.file_entry.take().unwrap().from_exit;
            app.mode = if from_exit { Mode::Exit } else { Mode::Edit };
            app.message.clear();
            return false;
        }
        if key.code == KeyCode::Enter {
            return app.submit_file();
        }
        let entry = app.file_entry.as_mut().unwrap();
        match key.code {
            KeyCode::Tab => {
                let matches = app.files.complete(&entry.path());
                if entry.completed {
                    entry.candidates = Some(matches.iter().map(|(p, _)| p.clone()).collect());
                    entry.candidate_scroll = 0;
                } else {
                    let completed = files::common_prefix(&matches);
                    if !matches.is_empty() {
                        entry.text = completed.chars().collect();
                        entry.cursor = entry.text.len();
                    }
                    entry.completed = true;
                    entry.candidates = if matches.is_empty() {
                        Some(Vec::new())
                    } else {
                        None
                    };
                }
            }
            KeyCode::Left => entry.cursor = entry.cursor.saturating_sub(1),
            KeyCode::Right => entry.cursor = (entry.cursor + 1).min(entry.text.len()),
            KeyCode::Up if entry.candidates.is_some() => {
                entry.candidate_scroll = entry.candidate_scroll.saturating_sub(1)
            }
            KeyCode::Down if entry.candidates.is_some() => {
                entry.candidate_scroll = entry.candidate_scroll.saturating_add(1)
            }
            KeyCode::Home => entry.cursor = 0,
            KeyCode::End => entry.cursor = entry.text.len(),
            KeyCode::Char('u') if ctrl => {
                entry.text.clear();
                entry.cursor = 0;
                entry.edited();
            }
            KeyCode::Backspace if entry.cursor > 0 => {
                entry.cursor -= 1;
                entry.text.remove(entry.cursor);
                entry.edited();
            }
            KeyCode::Delete if entry.cursor < entry.text.len() => {
                entry.text.remove(entry.cursor);
                entry.edited();
            }
            KeyCode::Char(c)
                if !ctrl
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::ALT | KeyModifiers::SUPER) =>
            {
                entry.text.insert(entry.cursor, c);
                entry.cursor += 1;
                entry.edited();
            }
            _ => {}
        }
        return false;
    }
    if app.mode == Mode::Export {
        match key.code {
            KeyCode::Char('x') if ctrl => {
                app.mode = Mode::Edit;
                app.message.clear();
            }
            KeyCode::Char('p' | 'P') if !ctrl && app.retained.is_some() => {
                app.open_file(FileType::Png, false)
            }
            KeyCode::Char('s' | 'S') if !ctrl && app.retained.is_some() => {
                app.open_file(FileType::Svg, false)
            }
            _ => {}
        }
        return false;
    }
    if app.mode == Mode::Exit {
        match key.code {
            KeyCode::Char('n' | 'N') if !ctrl => return true,
            KeyCode::Char('y' | 'Y') if !ctrl => app.open_file(FileType::Source, true),
            KeyCode::Char('x') if ctrl => app.mode = Mode::Edit,
            _ => {}
        }
        return false;
    }
    if app.mode == Mode::Find {
        match key.code {
            KeyCode::Char('x') if ctrl => {
                app.mode = Mode::Edit;
                app.message.clear();
            }
            KeyCode::Enter => app.find_next(),
            KeyCode::Backspace => {
                app.search.pop();
                app.message.clear();
            }
            KeyCode::Char(c) if !ctrl => {
                app.search.push(c);
                app.message.clear();
            }
            _ => {}
        }
        return false;
    }
    match key.code {
        KeyCode::Char('x') if ctrl => {
            if app.source() == app.initial {
                return true;
            }
            app.mode = Mode::Exit;
        }
        KeyCode::Char('f') if ctrl => {
            app.single_preview = false;
            app.mode = Mode::Find;
            app.search.clear();
        }
        KeyCode::Char('s') if ctrl && !app.single_preview => app.open_file(FileType::Source, false),
        KeyCode::Char('e') if ctrl && (!app.opts.single || app.single_preview) => {
            app.mode = Mode::Export;
            app.message.clear();
        }
        KeyCode::Char('n') if ctrl && !app.single_preview => app.insert("\n"),
        KeyCode::Enter if app.single_preview => app.single_preview = false,
        KeyCode::Enter => {
            app.start(width);
            if app.opts.single {
                app.single_preview = true;
            }
        }
        KeyCode::PageUp => app.y = app.y.saturating_sub(80),
        KeyCode::PageDown => app.y = app.y.saturating_add(80),
        KeyCode::Home if ctrl => app.x = app.x.saturating_sub(80),
        KeyCode::End if ctrl => app.x = app.x.saturating_add(80),
        KeyCode::Left if app.single_preview => app.x = app.x.saturating_sub(40),
        KeyCode::Right if app.single_preview => app.x = app.x.saturating_add(40),
        KeyCode::Up if app.single_preview => app.y = app.y.saturating_sub(40),
        KeyCode::Down if app.single_preview => app.y = app.y.saturating_add(40),
        KeyCode::Left => app.cursor = app.cursor.saturating_sub(1),
        KeyCode::Right => app.cursor = (app.cursor + 1).min(app.chars.len()),
        KeyCode::Up => app.vertical(false),
        KeyCode::Down => app.vertical(true),
        KeyCode::Backspace if !app.single_preview && app.cursor > 0 => {
            app.cursor -= 1;
            app.chars.remove(app.cursor);
            app.changed();
        }
        KeyCode::Delete if !app.single_preview && app.cursor < app.chars.len() => {
            app.chars.remove(app.cursor);
            app.changed();
        }
        KeyCode::Tab if !app.single_preview => app.insert(" "),
        KeyCode::Char(c)
            if !ctrl
                && !key
                    .modifiers
                    .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
                && !app.single_preview =>
        {
            app.insert(&c.to_string())
        }
        _ => {}
    }
    false
}
fn areas(
    area: Rect,
    single: bool,
    preview: bool,
    entry_lines: u16,
    message_lines: u16,
) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
    let [windows, latex, message, entry, nav] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(message_lines),
        Constraint::Length(entry_lines),
        Constraint::Length(1),
    ])
    .areas(area);
    let (editor, preview_area) = if single {
        if preview {
            (Rect::default(), windows)
        } else {
            (windows, Rect::default())
        }
    } else {
        let [a, b] = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(windows);
        (a, b)
    };
    (editor, preview_area, latex, message, entry, nav)
}
fn preview_width(area: Rect, picker: &Picker) -> u32 {
    u32::from(area.width.saturating_sub(2).max(1)) * u32::from(picker.font_size().width.max(1))
}
fn candidate_text(entry: &FileEntry) -> Option<String> {
    entry.candidates.as_ref().map(|candidates| {
        candidates
            .iter()
            .map(|path| {
                let is_dir = path.ends_with('/');
                let name = path
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(path);
                format!("{name}{}", if is_dir { "/" } else { "" })
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}
fn entry_lines(app: &App, area: Rect) -> u16 {
    match app.mode {
        Mode::Edit => 0,
        Mode::File => {
            let extra = app
                .file_entry
                .as_ref()
                .and_then(candidate_text)
                .map_or(0, |text| {
                    let lines = Paragraph::new(text)
                        .wrap(Wrap { trim: false })
                        .line_count(area.width.max(1))
                        .max(1);
                    let available = area
                        .height
                        .saturating_sub(message_lines(app, area.width).saturating_add(4));
                    (lines.min(5) as u16).min(available)
                });
            2 + extra
        }
        _ => 1,
    }
}
fn interaction_message(app: &App) -> String {
    match app.mode {
        Mode::Exit => "W501 WARNING: Answering \"No\" will destroy changes made".to_owned(),
        Mode::Export => app.export_warning().unwrap_or("").to_owned(),
        Mode::File => {
            let entry = app.file_entry.as_ref().unwrap();
            if let Some(error) = &entry.error {
                return error.clone();
            }
            match app.files.validate(&entry.path(), entry.kind) {
                Err(error) => error.to_owned(),
                Ok((_, mut warnings)) => {
                    if entry.kind != FileType::Source {
                        if let Some(warning) = app.export_warning() {
                            warnings.push(warning);
                        }
                    }
                    warnings.join("\n")
                }
            }
        }
        _ => app.message.clone(),
    }
}
fn message_lines(app: &App, width: u16) -> u16 {
    Paragraph::new(interaction_message(app))
        .wrap(Wrap { trim: false })
        .line_count(width.max(1))
        .clamp(1, usize::from(u16::MAX)) as u16
}
fn visible_path(entry: &FileEntry, width: u16) -> (String, u16) {
    let available = usize::from(width.saturating_sub(15));
    if available == 0 {
        return (String::new(), 0);
    }
    let mut start = 0;
    let mut before: usize = entry.text[..entry.cursor]
        .iter()
        .map(|c| c.width().unwrap_or(0))
        .sum();
    while before >= available && start < entry.cursor {
        before = before.saturating_sub(entry.text[start].width().unwrap_or(0));
        start += 1;
    }
    let mut shown = String::new();
    let mut used = 0;
    for c in &entry.text[start..] {
        let w = c.width().unwrap_or(0);
        if used + w > available {
            break;
        }
        shown.push(*c);
        used += w;
    }
    (shown, before as u16)
}

fn help_text(app: &App) -> String {
    let mut lines = vec!["Contextual shortcuts", ""];
    match app.mode {
        Mode::Edit if app.single_preview => {
            lines.extend([
                "Preview",
                "Enter          Return to the editor",
                "Arrow keys     Scroll the preview",
                "Page Up/Down   Scroll the preview vertically",
                "Ctrl+Home/End  Scroll the preview horizontally",
                "Ctrl+E         Export the retained result",
                "Ctrl+X         Exit",
            ]);
        }
        Mode::Edit => {
            lines.extend([
                "Editor",
                "Arrow keys     Move the cursor",
                "Ctrl+N         Insert a newline",
                "Tab            Insert a space",
                "Backspace/Del  Delete text",
            ]);
            lines.push(if app.opts.single {
                "Enter          Render and show the preview"
            } else {
                "Enter          Render the current source"
            });
            lines.extend(["Ctrl+F         Find text", "Ctrl+S         Save the source"]);
            if !app.opts.single {
                lines.extend([
                    "Ctrl+E         Export the retained result",
                    "Page Up/Down   Scroll the preview vertically",
                    "Ctrl+Home/End  Scroll the preview horizontally",
                ]);
            }
            lines.extend([
                "Ctrl+X         Exit",
                "",
                "Paste can include multiple lines.",
            ]);
        }
        Mode::Find => lines.extend([
            "Find",
            "Enter          Find the next match, wrapping at the end",
            "Backspace      Delete the last search character",
            "Ctrl+X         Close Find",
        ]),
        Mode::File => lines.extend([
            "File entry",
            "Tab            Complete the path; press again to list matches",
            "Up/Down        Scroll completion matches",
            "Left/Right     Move within the path",
            "Home/End       Move to the beginning/end of the path",
            "Backspace/Del  Delete path text",
            "Ctrl+U         Clear the path",
            "Enter          Accept warnings and write the file when valid",
            "Ctrl+X         Cancel",
        ]),
        Mode::Export => {
            lines.extend([
                "Export",
                "P              Choose PNG",
                "S              Choose SVG",
                "Ctrl+X         Cancel",
            ]);
            if app.retained.is_none() {
                lines.extend(["", "Render a preview before exporting."]);
            }
        }
        Mode::Exit => lines.extend([
            "Exit",
            "Y              Save the source and exit",
            "N              Discard changes and exit",
            "Ctrl+X         Cancel",
        ]),
    }
    lines.extend(["", "Ctrl+G         Open Help"]);
    lines.join("\n")
}

fn draw_help(frame: &mut ratatui::Frame, app: &mut App) {
    let text = help_text(app);
    let [title, content, nav] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let line_count = Paragraph::new(text.as_str())
        .wrap(Wrap { trim: false })
        .line_count(content.width.max(1));
    let help = app.help.as_mut().unwrap();
    help.page_height = content.height.max(1);
    help.scroll = help.scroll.min(
        line_count
            .saturating_sub(usize::from(content.height))
            .min(usize::from(u16::MAX)) as u16,
    );
    frame.render_widget(
        Paragraph::new("Help").style(Style::default().fg(Color::Cyan)),
        title,
    );
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((help.scroll, 0)),
        content,
    );
    frame.render_widget(
        Paragraph::new("^G Get Help   ↑↓ Scroll   PgUp/PgDn Page   ^X Return"),
        nav,
    );
}

fn draw(frame: &mut ratatui::Frame, app: &mut App, picker: &Picker) {
    if app.help.is_some() {
        draw_help(frame, app);
        return;
    }
    let (editor, preview, latex, message, entry, nav) = areas(
        frame.area(),
        app.opts.single,
        app.single_preview,
        entry_lines(app, frame.area()),
        message_lines(app, frame.area().width),
    );
    let too_small = if app.opts.single {
        editor.height.max(preview.height) < 3
    } else {
        editor.height < 3 || preview.height < 3
    };
    if editor.width > 0 && editor.height > 0 {
        let block = Block::default().title(" Editor ").borders(Borders::ALL);
        let inner = block.inner(editor);
        let source = app.source();
        let (row, _) = app.line_col();
        let start = app.chars[..app.cursor]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |i| i + 1);
        let before: String = app.chars[start..app.cursor].iter().collect();
        let col = UnicodeWidthStr::width(before.as_str());
        if inner.height > 0 && inner.width > 0 {
            let (mut sy, mut sx) = (
                usize::from(app.edit_scroll.0),
                usize::from(app.edit_scroll.1),
            );
            if row < sy {
                sy = row;
            } else if row >= sy + usize::from(inner.height) {
                sy = row + 1 - usize::from(inner.height);
            }
            if col < sx {
                sx = col;
            } else if col >= sx + usize::from(inner.width) {
                sx = col + 1 - usize::from(inner.width);
            }
            app.edit_scroll = (
                sy.min(u16::MAX as usize) as u16,
                sx.min(u16::MAX as usize) as u16,
            );
            frame.render_widget(
                Paragraph::new(source).block(block).scroll(app.edit_scroll),
                editor,
            );
            if app.mode == Mode::Edit {
                frame.set_cursor_position((
                    inner.x + (col - sx) as u16,
                    inner.y + (row - sy) as u16,
                ));
            }
        } else {
            frame.render_widget(block, editor);
        }
    }
    if preview.width > 0 && preview.height > 0 {
        let block = Block::default()
            .title(" Preview ")
            .title_bottom(Line::from(format!(" {} ", app.status())).right_aligned())
            .borders(Borders::ALL);
        let inner = block.inner(preview);
        frame.render_widget(block, preview);
        if let Some(img) = &app.preview {
            if inner.width > 0 && inner.height > 0 {
                let w = u32::from(inner.width) * u32::from(picker.font_size().width.max(1));
                let h = u32::from(inner.height) * u32::from(picker.font_size().height.max(1));
                app.x = app.x.min(img.width().saturating_sub(w));
                app.y = app.y.min(img.height().saturating_sub(h));
                let crop = img.crop_imm(
                    app.x,
                    app.y,
                    w.min(img.width() - app.x),
                    h.min(img.height() - app.y),
                );
                let mut state = picker.new_resize_protocol(crop);
                frame.render_stateful_widget(
                    StatefulImage::new().resize(Resize::Crop(None)),
                    inner,
                    &mut state,
                );
            }
        } else if !app.opts.live && app.job.is_none() && !app.failed {
            frame.render_widget(Paragraph::new("PRESS ENTER TO SEE PREVIEW"), inner);
        }
    }
    frame.render_widget(Paragraph::new(format!("LaTeX: {}", app.diagnostic)), latex);
    let msg = if too_small && app.mode == Mode::Edit {
        "W403 WARNING: Terminal too small. Please resize.".to_owned()
    } else {
        interaction_message(app)
    };
    frame.render_widget(
        Paragraph::new(msg)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Yellow)),
        message,
    );
    let (field, navigation) = match app.mode {
        Mode::Find => (
            format!("Search: {}", app.search),
            "^G Get Help   ^X Close   ENTER Next",
        ),
        Mode::Exit => (
            "Save modified buffer?  Y Yes   N No".into(),
            "^G Get Help   ^X Cancel",
        ),
        Mode::Export => (
            "Export preview as?  P png   S svg".into(),
            "^G Get Help   ^X Cancel",
        ),
        Mode::File => {
            let e = app.file_entry.as_ref().unwrap();
            let valid = app.files.validate(&e.path(), e.kind).is_ok();
            let (path, _) = visible_path(e, entry.width);
            (
                format!("File Type: {}\nFile to write: {}", e.kind.label(), path),
                if e.candidates.as_ref().is_some_and(|c| !c.is_empty()) && entry.height > 2 {
                    if valid {
                        "^G Get Help   ↑↓ Matches   TAB Complete   ^X Cancel   ENTER Complete"
                    } else {
                        "^G Get Help   ↑↓ Matches   TAB Complete   ^X Cancel"
                    }
                } else if valid {
                    "^G Get Help   TAB Complete path   ^X Cancel   ENTER Complete"
                } else {
                    "^G Get Help   TAB Complete path   ^X Cancel"
                },
            )
        }
        Mode::Edit => (
            String::new(),
            if app.single_preview {
                "^G Get Help   ENTER Editor   ^E Export   ^X Exit"
            } else if app.opts.single {
                "^G Get Help   ENTER Preview   ^F Find   ^S Save   ^X Exit"
            } else {
                "^G Get Help   ENTER Render   ^F Find   ^S Save   ^E Export   ^X Exit"
            },
        ),
    };
    frame.render_widget(Paragraph::new(field), entry);
    if app.mode == Mode::File && entry.height > 2 {
        let e = app.file_entry.as_mut().unwrap();
        if let Some(text) = candidate_text(e) {
            let area = Rect::new(entry.x, entry.y + 2, entry.width, entry.height - 2);
            let count = Paragraph::new(text.as_str())
                .wrap(Wrap { trim: false })
                .line_count(area.width.max(1));
            e.candidate_scroll = e.candidate_scroll.min(
                count
                    .saturating_sub(usize::from(area.height))
                    .min(usize::from(u16::MAX)) as u16,
            );
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .scroll((e.candidate_scroll, 0)),
                area,
            );
        }
    }
    if app.mode == Mode::File && entry.height >= 2 && entry.width > 15 {
        let e = app.file_entry.as_ref().unwrap();
        let (_, cursor) = visible_path(e, entry.width);
        frame.set_cursor_position((entry.x + 15 + cursor, entry.y + 1));
    }
    frame.render_widget(Paragraph::new(navigation), nav);
}

const MIN_COLUMNS: u16 = 80;
const MIN_ROWS: u16 = 24;

pub(crate) fn below_minimum(width: u16, height: u16) -> bool {
    width < MIN_COLUMNS || height < MIN_ROWS
}

pub(crate) fn draw_resize(frame: &mut ratatui::Frame) {
    let area = frame.area();
    let text = format!(
        "W403 WARNING: Terminal too small. Please resize.\nMinimum: {MIN_COLUMNS} columns and {MIN_ROWS} rows.\nCurrent size: {} columns and {} rows.",
        area.width, area.height
    );
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Yellow)),
        area,
    );
}

fn handle_event(app: &mut App, event: Event, width: u32, too_small: bool) -> bool {
    if too_small {
        return false;
    }
    match event {
        Event::Key(k) => key(app, k, width),
        Event::Paste(s) if app.help.is_none() && app.mode == Mode::Edit && !app.single_preview => {
            app.insert(&s.replace("\r\n", "\n").replace('\r', "\n"));
            false
        }
        Event::Paste(s) if app.help.is_none() && app.mode == Mode::Find => {
            app.search.push_str(&s);
            false
        }
        Event::Paste(s) if app.help.is_none() && app.mode == Mode::File => {
            let entry = app.file_entry.as_mut().unwrap();
            let text: Vec<char> = s.chars().filter(|c| *c != '\r' && *c != '\n').collect();
            let len = text.len();
            entry.text.splice(entry.cursor..entry.cursor, text);
            entry.cursor += len;
            entry.edited();
            false
        }
        _ => false,
    }
}

pub fn run(
    opts: Options,
    source: String,
    persistence: Option<Store>,
    startup_message: Option<String>,
) -> io::Result<()> {
    let (picker, warning) = graphics::detect();
    let mut terminal = ratatui::try_init()?;
    if let Err(e) = execute!(io::stdout(), EnableBracketedPaste) {
        ratatui::restore();
        return Err(e);
    }
    let mut app = App::new(opts, source);
    app.persistence = persistence;
    if let Some(message) = startup_message {
        app.message = message;
    }
    if app.opts.live {
        app.deadline = Some(Instant::now() + Duration::from_millis(300));
    }
    let result = (|| -> io::Result<()> {
        if let Some(warning) = warning {
            graphics::show_warning(&mut terminal, &warning)?;
        }
        loop {
            let size = terminal.size()?;
            let too_small = below_minimum(size.width, size.height);
            if too_small {
                app.finish();
                terminal.draw(draw_resize)?;
                if event::poll(Duration::from_millis(30))? {
                    handle_event(&mut app, event::read()?, 0, true);
                }
                continue;
            }
            let (_, p, _, _, _, _) = areas(
                Rect::new(0, 0, size.width, size.height),
                app.opts.single,
                app.single_preview,
                entry_lines(&app, Rect::new(0, 0, size.width, size.height)),
                message_lines(&app, size.width),
            );
            // In single Editor view, use the width that the Preview will have.
            let width = if p.width == 0 {
                u32::from(size.width.saturating_sub(2).max(1))
                    * u32::from(picker.font_size().width.max(1))
            } else {
                preview_width(p, &picker)
            };
            app.finish();
            app.finish_reflow(width);
            if app.deadline.is_some_and(|d| Instant::now() >= d) {
                app.start(width);
            }
            app.schedule_reflow(width);
            terminal.draw(|f| draw(f, &mut app, &picker))?;
            if event::poll(Duration::from_millis(30))?
                && handle_event(&mut app, event::read()?, width, false)
            {
                break;
            }
        }
        Ok(())
    })();
    if let Some(job) = app.job.take() {
        job.cancel();
    }
    if let Some(job) = app.reflow_job.take() {
        job.cancel();
    }
    let _ = execute!(io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    if let Some(warning) = app.exit_warning {
        eprintln!("{warning}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
        key(app, KeyEvent::new(code, modifiers), 400)
    }
    fn path(app: &mut App, path: &std::path::Path) {
        let entry = app.file_entry.as_mut().unwrap();
        entry.text = path.to_str().unwrap().chars().collect();
        entry.cursor = entry.text.len();
    }
    #[test]
    fn help_returns_to_exact_file_entry_and_ignores_paste() {
        let mut app = App::new(Options::default(), "original".into());
        app.cursor = 3;
        app.open_file(FileType::Source, true);
        let entry = app.file_entry.as_mut().unwrap();
        entry.text = "partially typed.txt".chars().collect();
        entry.cursor = 4;
        entry.candidates = Some(vec!["partial-one.txt".into(), "partial-two.txt".into()]);
        entry.candidate_scroll = 1;
        press(&mut app, KeyCode::Char('g'), KeyModifiers::CONTROL);
        assert!(app.help.is_some());
        assert!(help_text(&app).contains("File entry"));
        assert!(!handle_event(
            &mut app,
            Event::Paste("unexpected".into()),
            400,
            false
        ));
        press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(app.help.is_none());
        assert!(app.mode == Mode::File);
        let entry = app.file_entry.as_ref().unwrap();
        assert!(entry.from_exit);
        assert_eq!(entry.path(), "partially typed.txt");
        assert_eq!(entry.cursor, 4);
        assert_eq!(entry.candidate_scroll, 1);
        assert_eq!(app.cursor, 3);
        assert_eq!(app.source(), "original");
    }

    #[test]
    fn help_is_contextual_and_scrollable() {
        let mut app = App::new(
            Options {
                single: true,
                ..Options::default()
            },
            "x".into(),
        );
        app.single_preview = true;
        press(&mut app, KeyCode::Char('g'), KeyModifiers::CONTROL);
        let text = help_text(&app);
        assert!(text.contains("Preview"));
        assert!(text.contains("Ctrl+E"));
        assert!(!text.contains("Ctrl+F"));
        assert!(!text.contains("Ctrl+S"));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 5)).unwrap();
        terminal
            .draw(|frame| draw(frame, &mut app, &Picker::halfblocks()))
            .unwrap();
        press(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
        terminal
            .draw(|frame| draw(frame, &mut app, &Picker::halfblocks()))
            .unwrap();
        assert!(app.help.as_ref().unwrap().scroll > 0);
        press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(app.single_preview);
        assert!(app.help.is_none());
    }

    #[test]
    fn resize_screen_preserves_interaction_and_ignores_input() {
        assert!(below_minimum(79, 24));
        assert!(below_minimum(80, 23));
        assert!(!below_minimum(80, 24));
        let mut app = App::new(Options::default(), String::new());
        app.open_file(FileType::Source, false);
        let path = app.file_entry.as_ref().unwrap().path();
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!handle_event(&mut app, event, 0, true));
        assert!(!handle_event(
            &mut app,
            Event::Paste("lost".into()),
            0,
            true
        ));
        assert!(app.mode == Mode::File);
        assert_eq!(app.file_entry.as_ref().unwrap().path(), path);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(79, 24)).unwrap();
        terminal.draw(draw_resize).unwrap();
        let buffer = terminal.backend().buffer();
        let screen = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(screen.contains("W403"));
        assert!(screen.contains("80 columns and 24 rows"));
    }
    #[test]
    fn editor_handles_multiline_and_find_wraps() {
        let mut app = App::new(Options::default(), String::new());
        app.insert("x\ny x");
        app.vertical(false);
        assert_eq!(app.line_col(), (0, 1));
        app.search = "x".into();
        app.find_next();
        assert_eq!(app.cursor, 4);
        app.find_next();
        assert_eq!(app.cursor, 0);
        app.search = "missing".into();
        app.find_next();
        assert_eq!(app.message, "No matches");
    }
    #[test]
    fn status_tracks_retained_result_and_edits() {
        let mut app = App::new(Options::default(), "x".into());
        assert_eq!(app.status(), "Awaiting render");
        app.retained = Some("x".into());
        assert_eq!(app.status(), "Current");
        app.insert("y");
        assert_eq!(app.status(), "Outdated");
        app.failed = true;
        assert_eq!(app.status(), "Failed");
        app.insert("z");
        assert_eq!(app.status(), "Outdated");
    }
    #[test]
    fn reflow_preserves_retained_source_and_current_failure() {
        let mut app = App::new(
            Options {
                breaking: true,
                ..Options::default()
            },
            "new".into(),
        );
        app.retained = Some("old".into());
        app.render_width = Some(640);
        app.failed = true;
        app.diagnostic = "current render failed".into();
        app.apply_reflow("old", 800, 800, DynamicImage::new_rgba8(2, 2));
        assert_eq!(app.render_width, Some(800));
        assert_eq!(app.retained.as_deref(), Some("old"));
        assert_eq!(app.source(), "new");
        assert_eq!(app.status(), "Failed");
        assert_eq!(app.diagnostic, "current render failed");
        app.apply_reflow("old", 900, 1000, DynamicImage::new_rgba8(3, 3));
        app.apply_reflow("superseded", 1000, 1000, DynamicImage::new_rgba8(3, 3));
        assert_eq!(app.render_width, Some(800));
    }
    #[test]
    fn preview_does_not_edit_source() {
        let mut app = App::new(
            Options {
                single: true,
                ..Options::default()
            },
            "x".into(),
        );
        app.single_preview = true;
        key(
            &mut app,
            KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
            100,
        );
        assert_eq!(app.source(), "x");
    }

    #[test]
    fn save_and_exit_updates_dirty_baseline() {
        let dir = std::env::temp_dir().join(format!("rateq-ui-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("source.txt");
        let mut app = App::new(Options::default(), String::new());
        app.insert("x+1");
        assert!(!press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL));
        path(&mut app, &dest);
        assert!(!press(&mut app, KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "x+1");
        assert_eq!(app.initial, "x+1");
        app.insert("+2");
        press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
        press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
        assert!(app.file_entry.as_ref().unwrap().from_exit);
        assert!(press(&mut app, KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "x+1+2");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn successful_source_save_updates_resume_history() {
        let dir = std::env::temp_dir().join(format!(
            "rateq-ui-history-{}-{:x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let locations = crate::persistence::Locations {
            default_data: dir.join("data"),
            locator: dir.join("config/approved-location"),
        };
        let store = locations.approve(&locations.default_data).unwrap();
        let source = dir.join("source.txt");
        let mut app = App::new(Options::default(), String::new());
        app.persistence = Some(store.clone());
        app.insert("x+1");
        press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        path(&mut app, &source);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(store.last_source().unwrap(), Some(source.clone()));
        app.insert("+2");
        press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        path(&mut app, &dir.join("missing/source.txt"));
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(store.last_source().unwrap(), Some(source));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn history_failure_does_not_undo_successful_source_save() {
        let dir = std::env::temp_dir().join(format!(
            "rateq-ui-history-error-{}-{:x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let locations = crate::persistence::Locations {
            default_data: dir.join("data"),
            locator: dir.join("config/approved-location"),
        };
        let store = locations.approve(&locations.default_data).unwrap();
        std::fs::remove_dir(&locations.default_data).unwrap();
        let source = dir.join("source.txt");
        let mut app = App::new(Options::default(), String::new());
        app.persistence = Some(store);
        app.insert("x");
        press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        path(&mut app, &source);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "x");
        assert_eq!(app.initial, "x");
        assert!(app.message.contains("W206"));
        assert!(app.mode == Mode::Edit);
        app.insert("y");
        press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
        press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
        assert!(press(&mut app, KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.exit_warning.as_deref().unwrap().contains("W206"));
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "xy");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn exports_retained_source_and_blocks_unrendered_export() {
        let dir = std::env::temp_dir().join(format!("rateq-ui-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::new(Options::default(), "current".into());
        press(&mut app, KeyCode::Char('e'), KeyModifiers::CONTROL);
        assert!(app.export_warning().unwrap().contains("E301"));
        press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        assert!(app.mode == Mode::Export);
        app.retained = Some(r"\frac{1}{2}".into());
        assert!(app.export_warning().unwrap().contains("W205"));
        press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        let png = dir.join("old.png");
        path(&mut app, &png);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            std::fs::read(&png).unwrap(),
            render::export_png(r"\frac{1}{2}").unwrap()
        );
        press(&mut app, KeyCode::Char('e'), KeyModifiers::CONTROL);
        press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
        let svg = dir.join("old.svg");
        path(&mut app, &svg);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        let svg = std::fs::read_to_string(&svg).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<path"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn simultaneous_export_warnings_remain_visible() {
        let dir = std::env::temp_dir().join(format!("rateq warnings {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let output = dir.join("other.png");
        std::fs::write(&output, b"existing").unwrap();
        let mut app = App::new(Options::default(), "new".into());
        app.files = Files::new(dir.clone(), Some("original.txt".into()));
        app.retained = Some("old".into());
        app.open_file(FileType::Png, false);
        path(&mut app, &output);
        for width in [40, 60, 80] {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 30)).unwrap();
            terminal
                .draw(|frame| draw(frame, &mut app, &Picker::halfblocks()))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let screen: String = (0..buffer.area.height)
                .map(|y| {
                    (0..buffer.area.width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            for warning in ["W201", "W202", "W203", "W205"] {
                assert!(
                    screen.contains(warning),
                    "{warning} missing at width {width}:\n{screen}"
                );
            }
            let normalized = screen.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                normalized.contains("predates a failed render"),
                "W205 was clipped at width {width}:\n{screen}"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn minimum_size_fits_warnings_and_file_entry() {
        let dir = std::env::temp_dir().join(format!("rateq minimum {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let output = dir.join("other.png");
        std::fs::write(&output, b"existing").unwrap();
        let mut app = App::new(Options::default(), "new".into());
        app.files = Files::new(dir.clone(), Some("original.txt".into()));
        app.retained = Some("old".into());
        app.open_file(FileType::Png, false);
        path(&mut app, &output);
        app.file_entry.as_mut().unwrap().candidates =
            Some((0..12).map(|i| format!("item-{i:02}.png")).collect());
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(MIN_COLUMNS, MIN_ROWS))
                .unwrap();
        terminal
            .draw(|frame| draw(frame, &mut app, &Picker::halfblocks()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let screen = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        for value in [
            "W201",
            "W202",
            "W203",
            "W205",
            "File Type: PNG",
            "File to write:",
            "ENTER Complete",
        ] {
            assert!(
                screen.contains(value),
                "{value} missing at minimum size:\n{screen}"
            );
        }
        assert!(screen
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("predates a failed render"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn file_entry_updates_diagnostics_while_typing() {
        let dir = std::env::temp_dir().join(format!("rateq-ui-entry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let launch = dir.join("original.txt");
        std::fs::write(&launch, "original").unwrap();
        let mut app = App::new(
            Options {
                file: Some(launch),
                ..Options::default()
            },
            "original".into(),
        );
        press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(interaction_message(&app).contains("W201"));
        press(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert!(interaction_message(&app).contains("E203"));
        assert!(!press(&mut app, KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.mode == Mode::File);
        let changed = dir.join("renamed source.txt");
        for c in changed.to_str().unwrap().chars() {
            press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        let message = interaction_message(&app);
        assert!(message.contains("W202"));
        assert!(message.contains("W203"));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 24)).unwrap();
        terminal
            .draw(|frame| draw(frame, &mut app, &Picker::halfblocks()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let screen = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(screen.contains("W202"));
        assert!(screen.contains("W203"));
        assert!(!changed.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn completion_candidates_can_be_read_and_scrolled() {
        let dir = std::env::temp_dir().join(format!("rateq-ui-completion-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..12 {
            std::fs::write(dir.join(format!("alpha-{i:02}.txt")), "").unwrap();
        }
        let mut app = App::new(Options::default(), String::new());
        app.files = Files::new(dir.clone(), None);
        app.open_file(FileType::Source, false);
        path(&mut app, &dir.join("alpha-"));
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(
            app.file_entry
                .as_ref()
                .unwrap()
                .candidates
                .as_ref()
                .unwrap()
                .len(),
            12
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(50, 24)).unwrap();
        terminal
            .draw(|frame| draw(frame, &mut app, &Picker::halfblocks()))
            .unwrap();
        let screen = |terminal: &ratatui::Terminal<ratatui::backend::TestBackend>| {
            let buffer = terminal.backend().buffer();
            (0..buffer.area.height)
                .map(|y| {
                    (0..buffer.area.width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(screen(&terminal).contains("alpha-00.txt"));
        for _ in 0..20 {
            press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        }
        terminal
            .draw(|frame| draw(frame, &mut app, &Picker::halfblocks()))
            .unwrap();
        assert!(screen(&terminal).contains("alpha-11.txt"));
        press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(app.mode == Mode::Edit);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

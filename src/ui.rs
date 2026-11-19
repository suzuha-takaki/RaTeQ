use crate::{
    cli::Options,
    files::{self, FileType, Files},
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
    deadline: Option<Instant>,
    failed: bool,
    diagnostic: String,
    mode: Mode,
    files: Files,
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
            deadline: None,
            failed: false,
            diagnostic: String::new(),
            mode: Mode::Edit,
            files,
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
                        self.preview = Some(image);
                        self.retained = Some(job.source);
                        self.render_width = job.width;
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
    fn find(&mut self, backwards: bool) {
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
        self.cursor = if backwards {
            matches
                .iter()
                .rev()
                .find(|&&i| i < self.cursor)
                .copied()
                .unwrap_or(*matches.last().unwrap())
        } else {
            matches
                .iter()
                .find(|&&i| i > self.cursor)
                .copied()
                .unwrap_or(matches[0])
        };
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
        if kind == FileType::Source {
            self.initial = self.source();
        }
        self.file_entry = None;
        self.mode = Mode::Edit;
        self.message = format!("Wrote {}", path.display());
        from_exit
    }
}
fn key(app: &mut App, key: KeyEvent, width: u32) -> bool {
    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
        return false;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
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
            KeyCode::Enter => app.find(key.modifiers.contains(KeyModifiers::SHIFT)),
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
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) && !app.single_preview => {
            app.insert("\n")
        }
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
fn draw(frame: &mut ratatui::Frame, app: &mut App, picker: &Picker) {
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
        "W403 WARNING: Terminal too small to show the Editor/Preview area. Please resize."
            .to_owned()
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
        Mode::Find => (format!("Search: {}", app.search), "ENTER Next   ^X Close"),
        Mode::Exit => ("Save modified buffer?  Y Yes   N No".into(), "^X Cancel"),
        Mode::Export => ("Export preview as?  P png   S svg".into(), "^X Cancel"),
        Mode::File => {
            let e = app.file_entry.as_ref().unwrap();
            let valid = app.files.validate(&e.path(), e.kind).is_ok();
            let (path, _) = visible_path(e, entry.width);
            (
                format!("File Type: {}\nFile to write: {}", e.kind.label(), path),
                if e.candidates.as_ref().is_some_and(|c| !c.is_empty()) && entry.height > 2 {
                    if valid {
                        "↑↓ Matches   TAB Complete   ^X Cancel   ENTER Complete"
                    } else {
                        "↑↓ Matches   TAB Complete   ^X Cancel"
                    }
                } else if valid {
                    "TAB Complete path   ^X Cancel   ENTER Complete"
                } else {
                    "TAB Complete path   ^X Cancel"
                },
            )
        }
        Mode::Edit => (
            String::new(),
            if app.single_preview {
                "ENTER Editor   ^E Export   ^X Exit"
            } else if app.opts.single {
                "ENTER Preview   ^F Find   ^S Save   ^X Exit"
            } else {
                "ENTER Render   ^F Find   ^S Save   ^E Export   ^X Exit"
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

pub fn run(opts: Options, source: String) -> io::Result<()> {
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let mut terminal = ratatui::try_init()?;
    if let Err(e) = execute!(io::stdout(), EnableBracketedPaste) {
        ratatui::restore();
        return Err(e);
    }
    let mut app = App::new(opts, source);
    if app.opts.live {
        app.deadline = Some(Instant::now() + Duration::from_millis(300));
    }
    let result = (|| -> io::Result<()> {
        loop {
            let size = terminal.size()?;
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
            if app.deadline.is_some_and(|d| Instant::now() >= d) {
                app.start(width);
            }
            // Reflow a current retained result on resize; do not render an edited source in manual mode.
            if app.opts.breaking
                && app.job.is_none()
                && app.deadline.is_none()
                && !app.failed
                && app.retained.as_ref().is_some_and(|s| *s == app.source())
                && app.render_width != Some(width)
            {
                app.start(width);
            }
            terminal.draw(|f| draw(f, &mut app, &picker))?;
            if event::poll(Duration::from_millis(30))? {
                match event::read()? {
                    Event::Key(k) => {
                        if key(&mut app, k, width) {
                            break;
                        }
                    }
                    Event::Paste(s) if app.mode == Mode::Edit && !app.single_preview => {
                        app.insert(&s.replace("\r\n", "\n").replace('\r', "\n"))
                    }
                    Event::Paste(s) if app.mode == Mode::Find => app.search.push_str(&s),
                    Event::Paste(s) if app.mode == Mode::File => {
                        let entry = app.file_entry.as_mut().unwrap();
                        let text: Vec<char> =
                            s.chars().filter(|c| *c != '\r' && *c != '\n').collect();
                        let len = text.len();
                        entry.text.splice(entry.cursor..entry.cursor, text);
                        entry.cursor += len;
                        entry.edited();
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();
    if let Some(job) = app.job.take() {
        job.cancel();
    }
    let _ = execute!(io::stdout(), DisableBracketedPaste);
    ratatui::restore();
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
    fn editor_handles_multiline_and_find_wraps() {
        let mut app = App::new(Options::default(), String::new());
        app.insert("x\ny x");
        app.vertical(false);
        assert_eq!(app.line_col(), (0, 1));
        app.search = "x".into();
        app.find(false);
        assert_eq!(app.cursor, 4);
        app.find(false);
        assert_eq!(app.cursor, 0);
        app.search = "missing".into();
        app.find(false);
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

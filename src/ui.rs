use crate::{cli::Options, render};
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
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{picker::Picker, Resize, StatefulImage};
use std::{
    io::{self, Read, Write},
    process::{Child, Command, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use unicode_width::UnicodeWidthStr;

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
    search: String,
    message: String,
    single_preview: bool,
    x: u32,
    y: u32,
}
impl App {
    fn new(opts: Options, source: String) -> Self {
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
}
fn key(app: &mut App, key: KeyEvent, width: u32) -> bool {
    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
        return false;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if app.mode == Mode::Exit {
        match key.code {
            KeyCode::Char('n' | 'N') if !ctrl => return true,
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
    extra: bool,
) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
    let [windows, latex, message, entry, nav] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(u16::from(extra)),
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
fn draw(frame: &mut ratatui::Frame, app: &mut App, picker: &Picker) {
    let (editor, preview, latex, message, entry, nav) = areas(
        frame.area(),
        app.opts.single,
        app.single_preview,
        app.mode != Mode::Edit,
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
    let msg = if too_small {
        "W403 WARNING: Terminal too small to show the Editor/Preview area. Please resize."
    } else if app.mode == Mode::Exit {
        "W501 WARNING: Answering \"No\" will destroy changes made"
    } else {
        &app.message
    };
    frame.render_widget(
        Paragraph::new(msg).style(Style::default().fg(Color::Yellow)),
        message,
    );
    let (field, navigation) = match app.mode {
        Mode::Find => (format!("Search: {}", app.search), "ENTER Next   ^X Close"),
        Mode::Exit => (
            "Save modified buffer?  N No (discard); saving is not available yet".into(),
            "N Discard   ^X Cancel",
        ),
        Mode::Edit => (
            String::new(),
            if app.single_preview {
                "ENTER Editor   Arrows Scroll   ^X Exit"
            } else if app.opts.single {
                "ENTER Preview   ^F Find   ^N Newline   ^X Exit"
            } else {
                "ENTER Render   ^F Find   ^N Newline   ^X Exit"
            },
        ),
    };
    frame.render_widget(Paragraph::new(field), entry);
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
                app.mode != Mode::Edit,
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
}

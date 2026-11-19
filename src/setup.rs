use crate::{
    persistence::{Locations, Store},
    ui,
};
use ratatui::{
    crossterm::{
        event::{
            self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent,
            KeyEventKind, KeyModifiers,
        },
        execute,
    },
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use unicode_width::UnicodeWidthChar;

pub enum Outcome {
    Enabled(Store),
    Continue,
    Cancel,
}
#[derive(PartialEq)]
enum Screen {
    Choice,
    Custom,
    Confirm,
}
struct Setup {
    screen: Screen,
    custom: Vec<char>,
    cursor: usize,
    approved_path: Option<PathBuf>,
    scroll: u16,
    max_scroll: u16,
    error: String,
    help: bool,
}
enum Action {
    None,
    Approve(PathBuf),
    Continue,
    Cancel,
}
impl Setup {
    fn new(locations: &Locations) -> Self {
        let custom: Vec<_> = locations.default_data.to_string_lossy().chars().collect();
        Self {
            screen: Screen::Choice,
            cursor: custom.len(),
            custom,
            approved_path: None,
            scroll: 0,
            max_scroll: 0,
            error: String::new(),
            help: false,
        }
    }
    fn input(&mut self, key: KeyEvent, locations: &Locations) -> Action {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return Action::None;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let plain = !ctrl
            && !key
                .modifiers
                .intersects(KeyModifiers::ALT | KeyModifiers::SUPER);
        if self.help {
            if key.code == KeyCode::Char('x') && ctrl {
                self.help = false;
            }
            return Action::None;
        }
        if key.code == KeyCode::Char('g') && ctrl {
            self.help = true;
            return Action::None;
        }
        match self.screen {
            Screen::Choice => match key.code {
                KeyCode::Char('1') if plain => {
                    if self.scroll < self.max_scroll {
                        self.error = "Scroll down to review all locations before approving.".into();
                    } else {
                        return Action::Approve(locations.default_data.clone());
                    }
                }
                KeyCode::Char('2') if plain => {
                    self.screen = Screen::Custom;
                    self.error.clear();
                    self.scroll = 0;
                }
                KeyCode::Char('3') if plain => return Action::Continue,
                KeyCode::Char('x' | 'X') if !ctrl => return Action::Cancel,
                KeyCode::Char('x') if ctrl => return Action::Cancel,
                KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
                _ => {}
            },
            Screen::Custom => match key.code {
                KeyCode::Enter => {
                    let text: String = self.custom.iter().collect();
                    if text.is_empty() {
                        self.error = "E210 ERROR: Enter an application-data directory.".into();
                    } else {
                        let path = Path::new(&text);
                        match if path.is_absolute() {
                            Ok(path.to_path_buf())
                        } else {
                            std::env::current_dir().map(|cwd| cwd.join(path))
                        } {
                            Ok(path) => {
                                self.approved_path = Some(path);
                                self.screen = Screen::Confirm;
                                self.error.clear();
                                self.scroll = 0;
                            }
                            Err(e) => {
                                self.error = format!("E209 ERROR: Cannot set up persistence: {e}")
                            }
                        }
                    }
                }
                KeyCode::Char('x') if ctrl => {
                    self.screen = Screen::Choice;
                    self.error.clear();
                    self.scroll = 0;
                }
                KeyCode::Char('u') if ctrl => {
                    self.custom.clear();
                    self.cursor = 0;
                    self.error.clear();
                }
                KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
                KeyCode::Right => self.cursor = (self.cursor + 1).min(self.custom.len()),
                KeyCode::Home => self.cursor = 0,
                KeyCode::End => self.cursor = self.custom.len(),
                KeyCode::Backspace if self.cursor > 0 => {
                    self.cursor -= 1;
                    self.custom.remove(self.cursor);
                    self.error.clear();
                }
                KeyCode::Delete if self.cursor < self.custom.len() => {
                    self.custom.remove(self.cursor);
                    self.error.clear();
                }
                KeyCode::Char(c)
                    if !ctrl
                        && !key
                            .modifiers
                            .intersects(KeyModifiers::ALT | KeyModifiers::SUPER) =>
                {
                    self.custom.insert(self.cursor, c);
                    self.cursor += 1;
                    self.error.clear();
                }
                _ => {}
            },
            Screen::Confirm => match key.code {
                KeyCode::Char('y' | 'Y') if !ctrl => {
                    if self.scroll < self.max_scroll {
                        self.error = "Scroll down to review all locations before approving.".into();
                    } else {
                        return Action::Approve(self.approved_path.clone().unwrap());
                    }
                }
                KeyCode::Char('n' | 'N') if !ctrl => {
                    self.screen = Screen::Custom;
                    self.error.clear();
                    self.scroll = 0;
                }
                KeyCode::Char('x') if ctrl => {
                    self.screen = Screen::Choice;
                    self.error.clear();
                    self.scroll = 0;
                }
                KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
                _ => {}
            },
        }
        Action::None
    }
}

fn visible_input(setup: &Setup, width: u16) -> (String, u16) {
    let available = usize::from(width.saturating_sub(11));
    if available == 0 {
        return (String::new(), 0);
    }
    let mut start = 0;
    let mut before: usize = setup.custom[..setup.cursor]
        .iter()
        .map(|c| c.width().unwrap_or(0))
        .sum();
    while before >= available && start < setup.cursor {
        before = before.saturating_sub(setup.custom[start].width().unwrap_or(0));
        start += 1;
    }
    let mut text = String::new();
    let mut used = 0;
    for c in &setup.custom[start..] {
        let w = c.width().unwrap_or(0);
        if used + w > available {
            break;
        }
        text.push(*c);
        used += w;
    }
    (text, before as u16)
}

fn draw(frame: &mut ratatui::Frame, setup: &mut Setup, locations: &Locations) {
    if setup.help {
        let [title, body, nav] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(frame.area());
        let text = match setup.screen {
            Screen::Choice => "Persistence setup\n\n1        Approve the default locations\n2        Choose a custom application-data directory\n3        Continue without persistence\nX / Ctrl+X  Cancel and exit\nUp/Down  Scroll the proposed locations",
            Screen::Custom => "Custom location\n\nEnter       Review the chosen locations\nCtrl+U      Clear the directory field\nCtrl+X      Return to the setup choices\nLeft/Right  Move within the directory field\nHome/End    Move to the beginning or end\nBackspace/Delete  Delete path text",
            Screen::Confirm => "Confirm locations\n\nY        Approve the displayed locations\nN        Edit the directory\nCtrl+X   Return to the setup choices\nUp/Down  Scroll the proposed locations",
        };
        frame.render_widget(
            Paragraph::new("Help").style(Style::default().fg(Color::Cyan)),
            title,
        );
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), body);
        frame.render_widget(Paragraph::new("^G Get Help   ^X Return"), nav);
        return;
    }
    let error_lines = if setup.error.is_empty() {
        0
    } else {
        Paragraph::new(setup.error.as_str())
            .wrap(Wrap { trim: false })
            .line_count(frame.area().width.max(1))
            .min(4) as u16
    };
    let [title, intro, choices, body, input, error, nav] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(if setup.screen == Screen::Choice { 2 } else { 0 }),
        Constraint::Length(if setup.screen == Screen::Choice { 3 } else { 0 }),
        Constraint::Min(0),
        Constraint::Length(u16::from(setup.screen == Screen::Custom)),
        Constraint::Length(error_lines),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let heading = |text: &'static str| {
        Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
    };
    let (content, navigation) = match setup.screen {
        Screen::Choice => (vec![
            Line::from(""),
            Line::from("Use Up/Down to scroll the location details."),
            Line::from(""),
            heading("Default application-data directory:"),
            Line::from(locations.default_data.display().to_string()),
            Line::from(""),
            heading("Resume history:"),
            Line::from(Locations::history_path(&locations.default_data).display().to_string()),
            Line::from(""),
            heading("Location record:"),
            Line::from(locations.locator.display().to_string()),
        ], "^G Get Help   X / ^X Cancel"),
        Screen::Custom => (vec![
            Line::from("Choose the application-data directory. Enter an absolute path or a path relative to the launch directory."),
            Line::from(""),
            Line::from("A location record will also be written at:"),
            Line::from(locations.locator.display().to_string()),
            Line::from(""),
            Line::from("Press Enter to review both locations before approval."),
        ], "^G Help   ENTER Review   ^U Clear   ^X Back"),
        Screen::Confirm => (vec![
            Line::from("Approve persistence at these locations?"),
            Line::from(""),
            heading("Application-data directory:"),
            Line::from(setup.approved_path.as_ref().unwrap().display().to_string()),
            Line::from(""),
            heading("Resume history:"),
            Line::from(Locations::history_path(setup.approved_path.as_ref().unwrap()).display().to_string()),
            Line::from(""),
            heading("Location record:"),
            Line::from(locations.locator.display().to_string()),
            Line::from(""),
            Line::from("The location record is needed to find your chosen directory on later launches."),
            Line::from("Use Up/Down to scroll if needed."),
        ], "^G Help   Y Approve   N Edit path   ^X Back"),
    };
    let lines = Paragraph::new(content.clone())
        .wrap(Wrap { trim: false })
        .line_count(body.width.max(1));
    setup.max_scroll = lines
        .saturating_sub(usize::from(body.height))
        .min(usize::from(u16::MAX)) as u16;
    setup.scroll = setup.scroll.min(setup.max_scroll);
    frame.render_widget(
        Paragraph::new("Persistence setup").style(Style::default().fg(Color::Cyan)),
        title,
    );
    if setup.screen == Screen::Choice {
        frame.render_widget(
            Paragraph::new(
                "Resume needs permission to store the path of the last source file saved.",
            ),
            intro,
        );
        let number_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let options = [
            ("1", "  Use defaults"),
            ("2", "  Customize"),
            ("3", "  Continue without persistence"),
        ]
        .into_iter()
        .map(|(number, label)| {
            Line::from(vec![Span::styled(number, number_style), Span::raw(label)])
        })
        .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(options), choices);
    }
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .scroll((setup.scroll, 0)),
        body,
    );
    if setup.screen == Screen::Custom {
        let (text, cursor) = visible_input(setup, input.width);
        frame.render_widget(Paragraph::new(format!("Directory: {text}")), input);
        if input.height > 0 && input.width > 11 {
            frame.set_cursor_position((input.x + 11 + cursor, input.y));
        }
    }
    frame.render_widget(
        Paragraph::new(setup.error.as_str())
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Yellow)),
        error,
    );
    frame.render_widget(Paragraph::new(navigation), nav);
}

pub fn run(locations: &Locations) -> io::Result<Outcome> {
    let mut terminal = ratatui::try_init()?;
    if let Err(e) = execute!(io::stdout(), EnableBracketedPaste) {
        ratatui::restore();
        return Err(e);
    }
    let mut setup = Setup::new(locations);
    let result = (|| -> io::Result<Outcome> {
        loop {
            let size = terminal.size()?;
            if ui::below_minimum(size.width, size.height) {
                terminal.draw(ui::draw_resize)?;
                if event::poll(Duration::from_millis(50))? {
                    let _ = event::read()?;
                }
                continue;
            }
            terminal.draw(|frame| draw(frame, &mut setup, locations))?;
            if !event::poll(Duration::from_millis(50))? {
                continue;
            }
            match event::read()? {
                Event::Key(key) => match setup.input(key, locations) {
                    Action::None => {}
                    Action::Continue => return Ok(Outcome::Continue),
                    Action::Cancel => return Ok(Outcome::Cancel),
                    Action::Approve(path) => match locations.approve(&path) {
                        Ok(store) => return Ok(Outcome::Enabled(store)),
                        Err(e) => {
                            setup.error = format!("E209 ERROR: Cannot set up persistence: {e}")
                        }
                    },
                },
                Event::Paste(text) if setup.screen == Screen::Custom && !setup.help => {
                    let chars: Vec<_> = text.chars().filter(|c| *c != '\r' && *c != '\n').collect();
                    let len = chars.len();
                    setup.custom.splice(setup.cursor..setup.cursor, chars);
                    setup.cursor += len;
                    setup.error.clear();
                }
                _ => {}
            }
        }
    })();
    let _ = execute!(io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }
    #[test]
    fn choices_do_not_write_until_approved() {
        let root = std::env::temp_dir().join(format!(
            "rateq-setup-{}-{:x}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let locations = Locations {
            default_data: root.join("data"),
            locator: root.join("config/approved-location"),
        };
        let mut setup = Setup::new(&locations);
        assert!(matches!(
            setup.input(key('1'), &locations),
            Action::Approve(_)
        ));
        assert!(matches!(
            setup.input(key('3'), &locations),
            Action::Continue
        ));
        assert!(matches!(setup.input(key('x'), &locations), Action::Cancel));
        assert!(matches!(setup.input(key('2'), &locations), Action::None));
        assert!(matches!(
            setup.input(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                &locations
            ),
            Action::None
        ));
        assert!(setup.screen == Screen::Confirm);
        assert!(!root.exists());
        assert!(matches!(
            setup.input(key('y'), &locations),
            Action::Approve(_)
        ));
        assert!(!root.exists());
    }

    #[test]
    fn long_location_details_must_be_scrolled_before_approval() {
        let long = "directory-name/".repeat(100);
        let locations = Locations {
            default_data: PathBuf::from(format!("/tmp/{long}data")),
            locator: PathBuf::from(format!("/tmp/{long}config/approved-location")),
        };
        let mut setup = Setup::new(&locations);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| draw(frame, &mut setup, &locations))
            .unwrap();
        assert!(setup.max_scroll > 0);
        assert!(matches!(setup.input(key('1'), &locations), Action::None));
        assert!(setup.error.contains("review all locations"));
        for _ in 0..500 {
            setup.input(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &locations);
        }
        terminal
            .draw(|frame| draw(frame, &mut setup, &locations))
            .unwrap();
        assert!(matches!(
            setup.input(key('1'), &locations),
            Action::Approve(_)
        ));
    }

    #[test]
    fn help_preserves_custom_directory_input() {
        let locations = Locations {
            default_data: "/tmp/default".into(),
            locator: "/tmp/config/approved-location".into(),
        };
        let mut setup = Setup::new(&locations);
        setup.input(key('2'), &locations);
        setup.input(
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            &locations,
        );
        setup.input(key('a'), &locations);
        setup.input(
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            &locations,
        );
        assert!(setup.help);
        setup.input(key('z'), &locations);
        setup.input(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            &locations,
        );
        assert!(!setup.help);
        assert_eq!(setup.custom.iter().collect::<String>(), "a");
        assert_eq!(setup.cursor, 1);
        assert!(setup.screen == Screen::Custom);
    }
}

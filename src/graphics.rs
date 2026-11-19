use std::{
    env,
    io::{self, Write},
    os::fd::AsRawFd,
    time::{Duration, Instant},
};

use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Paragraph, Wrap},
    DefaultTerminal,
};
use ratatui_image::picker::{cap_parser::Parser, Picker, ProtocolType};

use crate::ui;

pub struct Warning {
    reason: &'static str,
    detected: ProtocolType,
    selected: ProtocolType,
}

// Kitty's basic graphics query does not establish support for the Unicode
// placeholders used by ratatui-image. Feature queries were added later, so a
// missing or old-style reply means unknown rather than unsupported.
fn placeholder_reply(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.split("\x1b_G").skip(1).any(|part| {
        let Some((reply, _)) = part.split_once("\x1b\\") else {
            return false;
        };
        let Some((keys, status)) = reply.split_once(';') else {
            return false;
        };
        let keys: Vec<_> = keys.split(',').collect();
        status == "OK" && keys.contains(&"i=32") && keys.contains(&"e=T") && keys.contains(&"U=1")
    })
}

struct RawMode;
impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn query_placeholders(tmux: bool) -> io::Result<bool> {
    enable_raw_mode()?;
    let _raw_mode = RawMode;
    let (start, escape, end) = Parser::tmux_start_escape_end(tmux);
    let query = format!("{start}{escape}_Ga=q,i=32,e=T,U=1{escape}\\{escape}[5n{end}");
    let mut stdout = io::stdout().lock();
    stdout.write_all(query.as_bytes())?;
    stdout.flush()?;
    drop(stdout);

    let stdin = io::stdin();
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut reply = Vec::new();
    while Instant::now() < deadline && reply.len() < 4096 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let mut poll = libc::pollfd {
            fd: stdin.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // poll only observes the stdin descriptor and does not take ownership.
        let ready = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().max(1) as i32) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if ready == 0 || poll.revents & libc::POLLIN == 0 {
            break;
        }
        let mut byte = [0];
        // Read directly from the descriptor: a buffered reader could retain
        // the rest of a reply while poll waits for bytes already in its buffer.
        let count = unsafe { libc::read(stdin.as_raw_fd(), byte.as_mut_ptr().cast(), 1) };
        if count < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if count == 0 {
            break;
        }
        reply.push(byte[0]);
        if reply.ends_with(b"\x1b[0n") {
            break;
        }
    }
    Ok(placeholder_reply(&reply))
}

pub fn detect() -> (Picker, Option<Warning>) {
    let result = Picker::from_query_stdio();
    let failed = result.is_err();
    let mut picker = result.unwrap_or_else(|_| Picker::halfblocks());
    let detected = picker.protocol_type();
    let reason = match detected {
        ProtocolType::Kitty if !query_placeholders(picker.tmux_detected()).unwrap_or(false) => {
            // WezTerm reports Kitty graphics but some versions do not support
            // the Unicode placements used by ratatui-image. WezTerm also
            // supports the iTerm2 protocol, including in remote sessions.
            if env::var("TERM_PROGRAM").is_ok_and(|value| value == "WezTerm") {
                picker.set_protocol_type(ProtocolType::Iterm2);
                None
            } else {
                Some("Kitty image placeholder support could not be confirmed.")
            }
        }
        ProtocolType::Halfblocks if failed => Some("Graphics detection did not complete."),
        ProtocolType::Halfblocks => Some("No terminal graphics protocol was confirmed."),
        _ => None,
    };
    let warning = reason.map(|reason| Warning {
        reason,
        detected,
        selected: picker.protocol_type(),
    });
    (picker, warning)
}

fn clean(value: &str) -> String {
    let value: String = value
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect();
    if value.is_empty() {
        "unavailable".into()
    } else {
        value
    }
}

fn variable(name: &str) -> String {
    env::var(name)
        .map(|s| clean(&s))
        .unwrap_or_else(|_| "unavailable".into())
}

fn os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
        {
            if output.status.success() {
                return format!("macOS {}", clean(&String::from_utf8_lossy(&output.stdout)));
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(info) = std::fs::read_to_string("/etc/os-release") {
            for line in info.lines() {
                if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
                    return clean(value.trim_matches('"').trim_matches('\''));
                }
            }
        }
    }
    format!("{} (version unavailable)", env::consts::OS)
}

fn report(warning: &Warning) -> Vec<String> {
    vec![
        format!("RaTeQ version: {}", env!("CARGO_PKG_VERSION")),
        format!("OS: {}", os_version()),
        format!("Terminal name: {}", variable("TERM_PROGRAM")),
        format!("Terminal version: {}", variable("TERM_PROGRAM_VERSION")),
        format!("Detected graphics protocol: {:?}", warning.detected),
        format!("Selected graphics protocol: {:?}", warning.selected),
        format!("Graphics check: {}", warning.reason),
        format!("TERM: {}", variable("TERM")),
        format!("TERM_PROGRAM: {}", variable("TERM_PROGRAM")),
        format!("TERM_PROGRAM_VERSION: {}", variable("TERM_PROGRAM_VERSION")),
        format!("COLORTERM: {}", variable("COLORTERM")),
        "Description / screenshot of the problem:".into(),
    ]
}

pub fn show_warning(terminal: &mut DefaultTerminal, warning: &Warning) -> io::Result<()> {
    let mut lines = vec![
        "Your terminal may not support the graphics features RaTeQ needs.".to_owned(),
        warning.reason.to_owned(),
        "RaTeQ will continue with the detected preview method.".to_owned(),
        String::new(),
        "If you encounter a problem, please report an issue at".to_owned(),
        "https://github.com/suzuha-takaki/RaTeQ/issues".to_owned(),
        String::new(),
        "Copy this template and add a description or screenshot:".to_owned(),
        "---".to_owned(),
    ];
    lines.extend(report(warning));
    lines.push("---".to_owned());
    let content = lines.into_iter().map(Line::from).collect::<Vec<_>>();
    let mut scroll = 0u16;
    loop {
        let size = terminal.size()?;
        if ui::below_minimum(size.width, size.height) {
            terminal.draw(ui::draw_resize)?;
            if event::poll(Duration::from_millis(50))? {
                let _ = event::read()?;
            }
            continue;
        }
        let mut max_scroll = 0;
        terminal.draw(|frame| {
            let [title, body, nav] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(frame.area());
            let paragraph = Paragraph::new(content.clone()).wrap(Wrap { trim: false });
            max_scroll = paragraph
                .line_count(body.width.max(1))
                .saturating_sub(usize::from(body.height))
                .min(usize::from(u16::MAX)) as u16;
            scroll = scroll.min(max_scroll);
            frame.render_widget(
                Paragraph::new("W401 Preview may not display correctly.")
                    .style(Style::default().fg(Color::Yellow)),
                title,
            );
            frame.render_widget(paragraph.scroll((scroll, 0)), body);
            frame.render_widget(Paragraph::new("Up/Down Scroll   ENTER Continue"), nav);
        })?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    continue;
                }
                match key.code {
                    KeyCode::Enter => return Ok(()),
                    KeyCode::Up => scroll = scroll.saturating_sub(1),
                    KeyCode::Down => scroll = scroll.saturating_add(1).min(max_scroll),
                    _ => {}
                }
            }
        }
    }
}

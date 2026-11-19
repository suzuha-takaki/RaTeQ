use crate::display;
use ratatui::{
    crossterm::{
        event::{
            self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste,
            EnableMouseCapture, Event, KeyCode, KeyEventKind,
        },
        execute,
    },
    widgets::Paragraph,
};
use std::{
    io,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn input_probe() -> io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let result = (|| -> io::Result<()> {
        let mut mouse = false;
        let mut paste = false;
        let mut events = Vec::<String>::new();
        loop {
            terminal.draw(|frame| {
                let title = format!("F10 quit | F2 mouse capture: {mouse} | F3 bracketed paste: {paste}\nTry keys, selection and multiline paste. Recent events:\n");
                frame.render_widget(Paragraph::new(title + &events.join("\n")), frame.area());
            })?;
            let event = event::read()?;
            if let Event::Key(key) = &event {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::F(10) => break,
                        KeyCode::F(2) => {
                            mouse = !mouse;
                            if mouse {
                                execute!(io::stdout(), EnableMouseCapture)?
                            } else {
                                execute!(io::stdout(), DisableMouseCapture)?
                            }
                        }
                        KeyCode::F(3) => {
                            paste = !paste;
                            if paste {
                                execute!(io::stdout(), EnableBracketedPaste)?
                            } else {
                                execute!(io::stdout(), DisableBracketedPaste)?
                            }
                        }
                        _ => {}
                    }
                }
            }
            events.push(format!("{event:?}"));
            if events.len() > 12 {
                events.remove(0);
            }
        }
        Ok(())
    })();
    let cleanup = execute!(io::stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();
    result.and(cleanup)
}

fn start_worker(revision: usize) -> io::Result<Child> {
    Command::new(std::env::current_exe()?)
        .args(["worker", &revision.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

// A child process makes cancellation measurable; RaTeX itself has no cancellation API here.
pub fn worker(revision: usize) -> Result<(), String> {
    let source = (0..100)
        .map(|i| format!(r"\frac{{x_{{{i}}}^2}}{{1+x_{{{i}}}}}"))
        .collect::<Vec<_>>()
        .join("+");
    let list = display(&format!("{source}+{revision}"))?;
    ratex_render::render_to_png(
        &list,
        &ratex_render::RenderOptions {
            font_size: 32.0,
            padding: 8.0,
            background_color: ratex_types::color::Color::new(0.0, 0.0, 0.0, 0.0),
            font_dir: String::new(),
            device_pixel_ratio: 2.0,
        },
    )?;
    println!("revision={revision}");
    Ok(())
}

pub fn scheduling() -> io::Result<()> {
    for cancel in [true, false] {
        let started = Instant::now();
        let mut active: Option<(usize, Child)> = None;
        let mut completed = 0;
        let mut stale = 0;
        let mut canceled = 0;
        let mut cancellation = Duration::ZERO;
        // Trigger a render, then simulate edits while it runs. After the burst,
        // wait the live-mode 300 ms debounce before rendering the latest source.
        for revision in 0..6 {
            if revision > 0 {
                thread::sleep(Duration::from_millis(10));
            }
            if let Some((rev, mut child)) = active.take() {
                if child.try_wait()?.is_some() {
                    let output = child.wait_with_output()?;
                    if output.status.success() {
                        if rev == revision {
                            completed += 1
                        } else {
                            stale += 1
                        }
                    } else {
                        return Err(io::Error::other(
                            String::from_utf8_lossy(&output.stderr).into_owned(),
                        ));
                    }
                } else if cancel {
                    let t = Instant::now();
                    child.kill()?;
                    child.wait()?;
                    cancellation += t.elapsed();
                    canceled += 1;
                } else {
                    active = Some((rev, child));
                }
            }
            if revision == 0 {
                active = Some((revision, start_worker(revision)?));
            }
        }
        let deadline = Instant::now() + Duration::from_millis(300);
        // Discard any old result regardless of success; never publish stale diagnostics.
        if let Some((_, child)) = active.take() {
            let output = child.wait_with_output()?;
            if !output.status.success() {
                return Err(io::Error::other(
                    String::from_utf8_lossy(&output.stderr).into_owned(),
                ));
            }
            stale += 1;
        }
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
        let output = start_worker(5)?.wait_with_output()?;
        if !output.status.success() {
            return Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        if String::from_utf8_lossy(&output.stdout).trim() != "revision=5" {
            return Err(io::Error::other("worker revision mismatch"));
        }
        completed += 1;
        println!("strategy={} published={completed} latest=5 stale_discarded={stale} canceled={canceled} cancellation_ms={:.3} elapsed_ms={:.3}", if cancel { "cancel" } else { "finish" }, cancellation.as_secs_f64()*1000.0, started.elapsed().as_secs_f64()*1000.0);
    }
    Ok(())
}

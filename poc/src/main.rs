mod equation;
mod experiments;

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use ratatui_image::{picker::Picker, StatefulImage};
use ratex_layout::{layout, to_display_list, validate_layout, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};
use ratex_svg::{render_to_svg, SvgOptions};
use ratex_types::{color::Color, display_item::DisplayList};
use std::{fs, path::Path, time::Instant};

fn display(source: &str) -> Result<DisplayList, String> {
    let ast = parse(source).map_err(|e| e.to_string())?;
    let layout = layout(&ast, &LayoutOptions::default().with_color(Color::BLACK));
    validate_layout(&layout)?;
    Ok(to_display_list(&layout))
}

fn run(name: &str, source: &str, out: &Path) -> Result<(), String> {
    let start = Instant::now();
    let list = display(source)?;
    let layout_time = start.elapsed();
    let png = render_to_png(
        &list,
        &RenderOptions {
            font_size: 32.0,
            padding: 8.0,
            background_color: Color::new(0.0, 0.0, 0.0, 0.0),
            font_dir: String::new(),
            device_pixel_ratio: 2.0,
        },
    )?;
    let png_time = start.elapsed();
    let svg = render_to_svg(
        &list,
        &SvgOptions {
            font_size: 32.0,
            padding: 8.0,
            embed_glyphs: true,
            ..Default::default()
        },
    );
    let svg_time = start.elapsed();
    let image = image::load_from_memory(&png).map_err(|e| e.to_string())?;
    let dimensions = (image.width(), image.height());
    let _protocol = Picker::halfblocks().new_resize_protocol(image);
    fs::write(out.join(format!("{name}.png")), png).map_err(|e| e.to_string())?;
    fs::write(out.join(format!("{name}.svg")), &svg).map_err(|e| e.to_string())?;
    println!("{name}: parse+layout={:.3}ms png={:.3}ms svg={:.3}ms dimensions={dimensions:?} svg_text={} svg_path={}", layout_time.as_secs_f64()*1000.0, (png_time-layout_time).as_secs_f64()*1000.0, (svg_time-png_time).as_secs_f64()*1000.0, svg.contains("<text"), svg.contains("<path"));
    Ok(())
}
fn preview(path: &str, halfblocks: bool) -> Result<(), Box<dyn std::error::Error>> {
    let image = image::open(path)?;
    // The export is full resolution; scale each preview dimension to one quarter.
    let image = image.resize_exact(
        (image.width() / 2).max(1),
        (image.height() / 2).max(1),
        image::imageops::FilterType::Lanczos3,
    );
    // Render transparent black equations against white, independent of the terminal theme.
    let mut background = image::RgbaImage::from_pixel(
        image.width(),
        image.height(),
        image::Rgba([255, 255, 255, 255]),
    );
    image::imageops::overlay(&mut background, &image.to_rgba8(), 0, 0);
    let picker = if halfblocks {
        Picker::halfblocks()
    } else {
        Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
    };
    let protocol = format!("{:?}", picker.protocol_type());
    let mut image_state = picker.new_resize_protocol(image::DynamicImage::ImageRgba8(background));
    let mut visible = true;
    let mut terminal = ratatui::try_init()?;
    let result = (|| -> std::io::Result<()> {
        loop {
            terminal.draw(|frame| {
                let [header, area] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)])
                    .areas(frame.area());
                frame.render_widget(
                    Paragraph::new(format!("{protocol} | {path} | space: clear/show | q: quit")),
                    header,
                );
                if visible {
                    frame.render_stateful_widget(StatefulImage::new(), area, &mut image_state);
                }
            })?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    break;
                }
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char(' ') {
                    visible = !visible;
                }
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(command) = args.first().map(String::as_str) {
        match command {
            "equation" => {
                if let Err(e) = equation::execute(&args[1..]) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
                return;
            }
            "input" if args.len() == 1 => {
                if let Err(e) = experiments::input_probe() {
                    eprintln!("Input probe failed: {e}");
                    std::process::exit(1);
                }
                return;
            }
            "schedule" if args.len() == 1 => {
                if let Err(e) = experiments::scheduling() {
                    eprintln!("Scheduling experiment failed: {e}");
                    std::process::exit(1);
                }
                return;
            }
            "worker" if args.len() == 2 => {
                let result = args[1]
                    .parse::<usize>()
                    .map_err(|e| e.to_string())
                    .and_then(experiments::worker);
                if let Err(e) = result {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
                return;
            }
            _ => {}
        }
    }
    if args.first().is_some_and(|s| s == "preview") {
        if !(2..=3).contains(&args.len()) || (args.len() == 3 && args[2] != "--halfblocks") {
            eprintln!("Usage: rateq-poc preview <png> [--halfblocks]");
            std::process::exit(2);
        }
        if let Err(error) = preview(&args[1], args.len() == 3) {
            eprintln!("Preview failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    let out = args
        .into_iter()
        .next()
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/output").into());
    let out = Path::new(&out);
    fs::create_dir_all(out).unwrap();
    let large = (0..100)
        .map(|i| format!(r"\frac{{x_{{{i}}}^2}}{{1+x_{{{i}}}}}"))
        .collect::<Vec<_>>()
        .join("+");
    let cases = [
        ("fraction", r"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"),
        ("matrix", r"\begin{pmatrix}a&b\\c&d\end{pmatrix}"),
        ("multiline", r"\begin{aligned}x&=1\\y&=x+2\end{aligned}"),
        ("unsupported", r"\notarealcommand{x}"),
        ("malformed", r"\frac{x"),
        ("large", large.as_str()),
    ];
    for pass in ["cold", "warm"] {
        println!("{pass}:");
        for (name, source) in cases {
            if let Err(e) = run(name, source, out) {
                println!("{name}: ERROR {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_and_invalid_sources() {
        assert!(display(r"\frac{1}{2}").is_ok());
        assert!(display(r"\notarealcommand{x}").is_err());
        assert!(display(r"\frac{x").is_err());
    }
}

use crate::{display, run};
use image::{DynamicImage, ImageBuffer, Rgba};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    widgets::Paragraph,
};
use ratatui_image::{picker::Picker, Resize, StatefulImage};
use ratex_layout::{layout, to_display_list, validate_layout, LayoutOptions};
use ratex_parser::{
    parse_node::{AtomFamily, Mode, ParseNode},
    parser::parse,
};
use ratex_render::{render_to_png, RenderOptions};
use ratex_types::color::Color;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn source(name: &str) -> Option<String> {
    match name {
        "fraction" => Some(r"\frac{-b \pm \sqrt{b^2-4ac}}{2a}".into()),
        "matrix" => Some(r"\begin{pmatrix}a&b\\c&d\end{pmatrix}".into()),
        "multiline" => Some(r"\begin{aligned}x&=1\\y&=x+2\end{aligned}".into()),
        "large" => Some(
            (0..100)
                .map(|i| format!(r"\frac{{x_{{{i}}}^2}}{{1+x_{{{i}}}}}"))
                .collect::<Vec<_>>()
                .join("+"),
        ),
        _ => None,
    }
}

#[derive(Debug, PartialEq)]
enum Action {
    Export,
    PreviewScroll,
    PreviewBreak,
}
fn arguments(args: &[String]) -> Result<(Vec<String>, Action), String> {
    let mut names = Vec::new();
    let mut action = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-i" => {
                i += 1;
                let name = args.get(i).ok_or("-i requires an equation name")?;
                if source(name).is_none() {
                    return Err(format!(
                        "Unknown equation: {name}. Available: fraction, matrix, multiline, large"
                    ));
                }
                names.push(name.clone());
            }
            "--export" => {
                if action.replace(Action::Export).is_some() {
                    return Err("Choose exactly one action".into());
                }
            }
            "--preview" => {
                i += 1;
                let mode = args.get(i).ok_or("--preview requires scroll or break")?;
                let next = match mode.as_str() {
                    "scroll" => Action::PreviewScroll,
                    "break" => Action::PreviewBreak,
                    _ => return Err(format!("Unknown preview mode: {mode}")),
                };
                if action.replace(next).is_some() {
                    return Err("Choose exactly one action".into());
                }
            }
            x => return Err(format!("Unknown argument: {x}")),
        }
        i += 1;
    }
    if names.is_empty() {
        return Err("At least one -i is required".into());
    }
    Ok((
        names,
        action.ok_or("Choose --export or --preview scroll|break")?,
    ))
}

// Only break before a top-level binary +. Nested groups, environments and
// existing explicit line breaks are untouched. Width is measured by RaTeX.
fn wrapped(source: &str, max_width: f64) -> Result<(Vec<ParseNode>, usize), String> {
    let nodes = parse(source).map_err(|e| e.to_string())?;
    if nodes.iter().any(|n| matches!(n, ParseNode::Cr { .. })) {
        return Ok((nodes, 0));
    }
    let opts = LayoutOptions::default().with_color(Color::BLACK);
    let mut output = Vec::new();
    let mut row = Vec::new();
    let mut breaks = 0;
    for node in nodes {
        row.push(node);
        if layout(&row, &opts).width > max_width {
            if let Some(pos) = row.iter().enumerate().skip(1).rev().find_map(|(i, n)| {
                if matches!(n, ParseNode::Atom { family: AtomFamily::Bin, text, .. } if text == "+")
                {
                    Some(i)
                } else {
                    None
                }
            }) {
                let remainder = row.split_off(pos);
                output.append(&mut row);
                output.push(ParseNode::Cr {
                    mode: Mode::Math,
                    new_line: true,
                    size: None,
                    loc: None,
                });
                breaks += 1;
                row = remainder;
            }
        }
    }
    output.append(&mut row);
    Ok((output, breaks))
}

fn preview_image(name: &str, width: u32, breaking: bool) -> Result<(DynamicImage, usize), String> {
    let src = source(name).ok_or("unknown equation")?;
    let (list, breaks) = if breaking {
        let (nodes, count) = wrapped(&src, (width.saturating_sub(16) as f64 / 32.0).max(0.1))?;
        let tree = layout(&nodes, &LayoutOptions::default().with_color(Color::BLACK));
        validate_layout(&tree)?;
        (to_display_list(&tree), count)
    } else {
        (display(&src)?, 0)
    };
    let png = render_to_png(
        &list,
        &RenderOptions {
            font_size: 32.0,
            padding: 8.0,
            background_color: Color::new(0.0, 0.0, 0.0, 0.0),
            font_dir: String::new(),
            device_pixel_ratio: 1.0,
        },
    )?;
    let img = image::load_from_memory(&png)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let mut white = ImageBuffer::from_pixel(img.width(), img.height(), Rgba([255, 255, 255, 255]));
    image::imageops::overlay(&mut white, &img, 0, 0);
    Ok((DynamicImage::ImageRgba8(white), breaks))
}

fn preview(names: &[String], breaking: bool) -> Result<(), Box<dyn std::error::Error>> {
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let font = picker.font_size();
    let protocol = format!("{:?}", picker.protocol_type());
    let mut terminal = ratatui::try_init()?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let (mut index, mut x, mut y) = (0usize, 0u32, 0u32);
        let mut cached: Option<(usize, u16, DynamicImage, usize)> = None;
        loop {
            let area = terminal.size()?;
            let cols = area.width.max(1);
            let rows = area.height.saturating_sub(2).max(1);
            let width = u32::from(cols) * u32::from(font.width.max(1));
            let height = u32::from(rows) * u32::from(font.height.max(1));
            if cached
                .as_ref()
                .is_none_or(|(idx, w, _, _)| *idx != index || (breaking && *w != cols))
            {
                let (img, breaks) =
                    preview_image(&names[index], width, breaking).map_err(std::io::Error::other)?;
                cached = Some((index, cols, img, breaks));
                x = 0;
                y = 0;
            }
            let (_, _, img, breaks) = cached.as_ref().unwrap();
            x = x.min(img.width().saturating_sub(width));
            y = y.min(img.height().saturating_sub(height));
            let crop = img.crop_imm(
                x,
                y,
                width.min(img.width() - x),
                height.min(img.height() - y),
            );
            let mut state = picker.new_resize_protocol(crop);
            terminal.draw(|f| {
                let [header, content, footer] = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .areas(f.area());
                f.render_widget(
                    Paragraph::new(format!(
                        "{} ({}/{}) | {} | {} | breaks: {}",
                        names[index],
                        index + 1,
                        names.len(),
                        if breaking { "break" } else { "scroll" },
                        protocol,
                        breaks
                    )),
                    header,
                );
                f.render_stateful_widget(
                    StatefulImage::new().resize(Resize::Crop(None)),
                    content,
                    &mut state,
                );
                f.render_widget(
                    Paragraph::new(format!(
                        "Arrows scroll | n/p next/previous | q quit | offset {x},{y} px"
                    )),
                    footer,
                );
            })?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('n') if index + 1 < names.len() => index += 1,
                        KeyCode::Char('p') if index > 0 => index -= 1,
                        KeyCode::Left => x = x.saturating_sub(u32::from(font.width.max(1)) * 4),
                        KeyCode::Right => x = x.saturating_add(u32::from(font.width.max(1)) * 4),
                        KeyCode::Up => y = y.saturating_sub(u32::from(font.height.max(1)) * 2),
                        KeyCode::Down => y = y.saturating_add(u32::from(font.height.max(1)) * 2),
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}

pub fn execute(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (names, action) = arguments(args).map_err(std::io::Error::other)?;
    if action == Action::Export {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("output")
            .join(format!("equations-{nonce}"));
        fs::create_dir_all(&dir)?;
        for (i, name) in names.iter().enumerate() {
            run(
                &format!("{:02}-{name}", i + 1),
                &source(name).unwrap(),
                &dir,
            )
            .map_err(std::io::Error::other)?;
        }
        println!("Exports: {}", dir.display());
    } else {
        preview(&names, action == Action::PreviewBreak)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_before_running() {
        assert!(arguments(&["-i".into(), "bad".into(), "--export".into()]).is_err());
        assert!(arguments(&[
            "-i".into(),
            "matrix".into(),
            "--export".into(),
            "--preview".into(),
            "scroll".into()
        ])
        .is_err());
    }
    #[test]
    fn breaks_only_at_top_level_and_preserves_nodes() {
        let (nodes, breaks) = wrapped(r"\frac{a+b}{c}+x+y", 1.0).unwrap();
        assert!(breaks > 0);
        assert_eq!(
            nodes
                .iter()
                .filter(|n| matches!(n, ParseNode::Cr { .. }))
                .count(),
            breaks
        );
        let mut unwrapped = nodes;
        unwrapped.retain(|n| !matches!(n, ParseNode::Cr { .. }));
        assert_eq!(
            format!("{unwrapped:?}"),
            format!("{:?}", parse(r"\frac{a+b}{c}+x+y").unwrap())
        );
    }
}

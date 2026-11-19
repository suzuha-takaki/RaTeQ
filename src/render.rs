use image::{DynamicImage, ImageBuffer, Rgba};
use ratex_layout::{layout, to_display_list, validate_layout, LayoutOptions};
use ratex_parser::{
    parse_node::{AtomFamily, Mode, ParseNode},
    parser::parse,
};
use ratex_render::{render_to_png, RenderOptions};
use ratex_types::color::Color;

// Break only at top-level binary plus signs, preserving nested groups and explicit breaks.
fn nodes(source: &str, width: Option<u32>) -> Result<Vec<ParseNode>, String> {
    let nodes = parse(source).map_err(|e| e.to_string())?;
    let Some(width) = width else { return Ok(nodes) };
    if nodes.iter().any(|n| matches!(n, ParseNode::Cr { .. })) {
        return Ok(nodes);
    }
    let max_width = (width.saturating_sub(16) as f64 / 32.0).max(0.1);
    let opts = LayoutOptions::default().with_color(Color::BLACK);
    let (mut output, mut row) = (Vec::new(), Vec::new());
    for node in nodes {
        row.push(node);
        if layout(&row, &opts).width > max_width {
            if let Some(pos) = row.iter().enumerate().skip(1).rev().find_map(|(i, n)|
                matches!(n, ParseNode::Atom { family: AtomFamily::Bin, text, .. } if text == "+").then_some(i)) {
                let rest = row.split_off(pos);
                output.append(&mut row);
                output.push(ParseNode::Cr { mode: Mode::Math, new_line: true, size: None, loc: None });
                row = rest;
            }
        }
    }
    output.append(&mut row);
    Ok(output)
}

pub fn png(source: &str, width: Option<u32>) -> Result<Vec<u8>, String> {
    let tree = layout(
        &nodes(source, width)?,
        &LayoutOptions::default().with_color(Color::BLACK),
    );
    validate_layout(&tree)?;
    render_to_png(
        &to_display_list(&tree),
        &RenderOptions {
            font_size: 32.0,
            padding: 8.0,
            background_color: Color::new(0.0, 0.0, 0.0, 0.0),
            font_dir: String::new(),
            device_pixel_ratio: 1.0,
        },
    )
}

pub fn white_image(png: &[u8]) -> Result<DynamicImage, String> {
    let img = image::load_from_memory(png)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let mut white = ImageBuffer::from_pixel(img.width(), img.height(), Rgba([255, 255, 255, 255]));
    image::imageops::overlay(&mut white, &img, 0, 0);
    Ok(DynamicImage::ImageRgba8(white))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_and_rejects_invalid_math() {
        assert!(white_image(&png(r"\frac{1}{2}", None).unwrap()).is_ok());
        assert!(png(r"\notarealcommand{x}", None).is_err());
    }
    #[test]
    fn breaking_preserves_non_break_nodes() {
        let mut broken = nodes(r"\frac{a+b}{c}+x+y", Some(32)).unwrap();
        assert!(broken.iter().any(|n| matches!(n, ParseNode::Cr { .. })));
        broken.retain(|n| !matches!(n, ParseNode::Cr { .. }));
        assert_eq!(
            format!("{broken:?}"),
            format!("{:?}", nodes(r"\frac{a+b}{c}+x+y", None).unwrap())
        );
    }
}

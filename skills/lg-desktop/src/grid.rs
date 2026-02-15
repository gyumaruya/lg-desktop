use std::env;
use std::path::Path;

use ab_glyph::FontRef;
use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};

const GRID_COLS: u32 = 10;
const GRID_ROWS: u32 = 10;
const GRID_COLOR: Rgba<u8> = Rgba([255, 0, 0, 180]);
const LABEL_COLOR: Rgba<u8> = Rgba([255, 255, 0, 255]);
const FONT_SCALE: f32 = 16.0;

/// Font search paths for Linux containers (Alpine, Ubuntu, Debian, Fedora).
const FONT_PATHS: &[&str] = &[
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansDisplay-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu-sans-mono-fonts/DejaVuSansMono.ttf",
];

fn grid_to_pixel(grid_ref: &str, img_width: u32, img_height: u32) -> Option<(u32, u32)> {
    let col = grid_ref.chars().next()?.to_ascii_uppercase() as u32 - 'A' as u32;
    let row: u32 = grid_ref[1..].parse::<u32>().ok()? - 1;
    if col >= GRID_COLS || row >= GRID_ROWS {
        return None;
    }
    let cell_w = img_width / GRID_COLS;
    let cell_h = img_height / GRID_ROWS;
    Some((col * cell_w + cell_w / 2, row * cell_h + cell_h / 2))
}

fn load_font() -> Option<Vec<u8>> {
    for path in FONT_PATHS {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }
    None
}

fn draw_grid(img: &mut RgbaImage) {
    let (w, h) = (img.width(), img.height());
    let cell_w = w as f32 / GRID_COLS as f32;
    let cell_h = h as f32 / GRID_ROWS as f32;

    // Draw vertical lines
    for i in 0..=GRID_COLS {
        let x = cell_w * i as f32;
        draw_line_segment_mut(img, (x, 0.0), (x, h as f32 - 1.0), GRID_COLOR);
    }

    // Draw horizontal lines
    for i in 0..=GRID_ROWS {
        let y = cell_h * i as f32;
        draw_line_segment_mut(img, (0.0, y), (w as f32 - 1.0, y), GRID_COLOR);
    }
}

fn draw_labels(img: &mut RgbaImage, font: &FontRef<'_>) {
    let (w, h) = (img.width(), img.height());
    let scale = ab_glyph::PxScale::from(FONT_SCALE);

    for col in 0..GRID_COLS {
        for row in 0..GRID_ROWS {
            let label = format!("{}{}", (b'A' + col as u8) as char, row + 1);
            if let Some((cx, cy)) = grid_to_pixel(&label, w, h) {
                // Offset label to top-left of center
                let x = i32::try_from(cx.saturating_sub(10)).unwrap_or(0);
                let y = i32::try_from(cy.saturating_sub(8)).unwrap_or(0);
                draw_text_mut(img, LABEL_COLOR, x, y, scale, font, &label);
            }
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: lg-grid <input-image> <output-image>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let img =
        image::open(input_path).with_context(|| format!("failed to open image: {input_path}"))?;
    let mut rgba = img.to_rgba8();

    draw_grid(&mut rgba);

    // Try to load a font for labels
    match load_font() {
        Some(font_data) => match FontRef::try_from_slice(&font_data) {
            Ok(font) => draw_labels(&mut rgba, &font),
            Err(e) => eprintln!("[lg-grid] warning: font data is invalid, grid drawn without labels: {e}"),
        },
        None => eprintln!("[lg-grid] warning: no font found at any search path, grid drawn without labels"),
    }

    rgba.save(Path::new(output_path))
        .with_context(|| format!("failed to save image: {output_path}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_to_pixel_a1() {
        assert_eq!(grid_to_pixel("A1", 1000, 1000), Some((50, 50)));
    }

    #[test]
    fn test_grid_to_pixel_j10() {
        assert_eq!(grid_to_pixel("J10", 1000, 1000), Some((950, 950)));
    }

    #[test]
    fn test_grid_to_pixel_center() {
        // E5 on 1000x1000: col=4, row=4, cell=100x100, center=(450, 450)
        assert_eq!(grid_to_pixel("E5", 1000, 1000), Some((450, 450)));
    }

    #[test]
    fn test_grid_to_pixel_out_of_range() {
        assert_eq!(grid_to_pixel("K1", 1000, 1000), None);
        assert_eq!(grid_to_pixel("A11", 1000, 1000), None);
    }

    #[test]
    fn test_grid_to_pixel_lowercase() {
        assert_eq!(grid_to_pixel("a1", 1000, 1000), Some((50, 50)));
    }

    #[test]
    fn test_grid_to_pixel_real_resolution() {
        // 1280x1024: cell_w=128, cell_h=102
        let (x, y) = grid_to_pixel("A1", 1280, 1024).unwrap();
        assert_eq!(x, 64);  // 128/2
        assert_eq!(y, 51);  // 102/2
    }
}

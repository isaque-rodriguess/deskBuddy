use image::{imageops::FilterType, Rgba, RgbaImage};
use std::path::Path;

pub const FRAME_W: u32 = 192;
pub const FRAME_H: u32 = 208;
pub const FRAMES_PER_STATE: usize = 6;
pub const LOOP_MS: u64 = 1100;

pub const STATE_ROWS: [&str; 9] = [
    "idle",
    "running-right",
    "running-left",
    "waving",
    "jumping",
    "failed",
    "waiting",
    "running",
    "review",
];

const ALIASES: [(&str, &str); 12] = [
    ("idle", "idle"),
    ("wave", "wave"),
    ("waving", "wave"),
    ("jump", "jump"),
    ("jumping", "jump"),
    ("run", "run"),
    ("running", "run"),
    ("running-right", "run"),
    ("running-left", "run"),
    ("failed", "failed"),
    ("review", "review"),
    ("waiting", "waiting"),
];

fn alias(state: &str) -> &str {
    ALIASES
        .iter()
        .find(|(k, _)| *k == state)
        .map(|(_, v)| *v)
        .unwrap_or(state)
}

pub fn state_row_index(state: &str, row_count: usize) -> usize {
    let canonical = alias(state);
    let rows = &STATE_ROWS[..row_count.min(STATE_ROWS.len())];
    for (i, row_name) in rows.iter().enumerate() {
        if *row_name == state || alias(row_name) == canonical {
            return i;
        }
    }
    0
}

fn is_blank(frame: &RgbaImage) -> bool {
    let max_alpha = frame.pixels().map(|p: &Rgba<u8>| p.0[3]).max().unwrap_or(0);
    max_alpha <= 8
}

pub fn load_frames_from_sheet(sheet: &RgbaImage, state: &str, scale: f32) -> Vec<RgbaImage> {
    let (sheet_w, sheet_h) = (sheet.width(), sheet.height());
    let rows = ((sheet_h / FRAME_H) as usize).max(1);
    let row_idx = state_row_index(state, rows);

    let mut top = (row_idx as u32) * FRAME_H;
    if top + FRAME_H > sheet_h {
        top = sheet_h.saturating_sub(FRAME_H);
    }

    let cols = ((sheet_w / FRAME_W) as usize).max(1);
    let count = FRAMES_PER_STATE.min(cols);

    let mut frames = Vec::with_capacity(count);
    for i in 0..count {
        let left = (i as u32) * FRAME_W;
        let mut frame = image::imageops::crop_imm(sheet, left, top, FRAME_W, FRAME_H).to_image();
        if is_blank(&frame) {
            break;
        }
        if (scale - 1.0).abs() > f32::EPSILON {
            let new_w = ((FRAME_W as f32) * scale).round() as u32;
            let new_h = ((FRAME_H as f32) * scale).round() as u32;
            frame = image::imageops::resize(&frame, new_w, new_h, FilterType::Lanczos3);
        }
        frames.push(frame);
    }
    frames
}

pub fn load_frames(
    state: &str,
    scale: f32,
    sheet_path: &Path,
) -> image::ImageResult<Vec<RgbaImage>> {
    let sheet = image::open(sheet_path)?.to_rgba8();
    Ok(load_frames_from_sheet(&sheet, state, scale))
}

pub fn get_frame_size(scale: f32) -> (u32, u32) {
    let w = ((FRAME_W as f32) * scale).round() as u32;
    let h = ((FRAME_H as f32) * scale).round() as u32;
    (w, h)
}

pub fn frame_interval_ms(state: &str, sheet_path: &Path) -> image::ImageResult<f64> {
    let frames = load_frames(state, 1.0, sheet_path)?;
    let count = frames.len().max(1);
    Ok((LOOP_MS as f64) / (count as f64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn zero_sheet() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/zero-2/spritesheet.webp")
    }

    #[test]
    fn state_row_index_known_states() {
        // Parity with Python sprite_parser.state_row_index (alias quirk included).
        assert_eq!(state_row_index("idle", 9), 0);
        assert_eq!(state_row_index("running-right", 9), 1);
        assert_eq!(state_row_index("waving", 9), 3);
        assert_eq!(state_row_index("jumping", 9), 4);
        assert_eq!(state_row_index("failed", 9), 5);
        assert_eq!(state_row_index("waiting", 9), 6);
        assert_eq!(state_row_index("review", 9), 8);
    }

    #[test]
    fn state_row_index_aliases() {
        // Alias "run" collapses running/running-right/running-left/run all to row 1.
        assert_eq!(state_row_index("running-left", 9), 1);
        assert_eq!(state_row_index("running", 9), 1);
        assert_eq!(state_row_index("run", 9), 1);
        assert_eq!(state_row_index("wave", 9), 3);
        assert_eq!(state_row_index("jump", 9), 4);
        // "run_right"/"run_left" are not in ALIASES → canonical == input → no match → 0.
        assert_eq!(state_row_index("run_right", 9), 0);
        assert_eq!(state_row_index("run_left", 9), 0);
        assert_eq!(state_row_index("unknown", 9), 0);
    }

    #[test]
    fn load_frames_idle_returns_frames() {
        let sheet = zero_sheet();
        if !sheet.exists() {
            eprintln!("skipping: spritesheet not present at {sheet:?}");
            return;
        }
        let frames = load_frames("idle", 1.0, &sheet).expect("load");
        assert!(!frames.is_empty(), "idle should have frames");
        assert!(frames.len() <= FRAMES_PER_STATE);
        for f in &frames {
            assert_eq!(f.dimensions(), (FRAME_W, FRAME_H));
        }
    }

    #[test]
    fn load_frames_with_scale() {
        let sheet = zero_sheet();
        if !sheet.exists() {
            eprintln!("skipping: spritesheet not present");
            return;
        }
        let frames = load_frames("idle", 0.5, &sheet).expect("load");
        assert!(!frames.is_empty());
        for f in &frames {
            assert_eq!(f.dimensions(), (96, 104));
        }
    }

    #[test]
    fn get_frame_size_scale() {
        assert_eq!(get_frame_size(1.0), (192, 208));
        assert_eq!(get_frame_size(0.5), (96, 104));
        assert_eq!(get_frame_size(0.7), (134, 146));
    }

    #[test]
    fn frame_interval_ms_idle() {
        let sheet = zero_sheet();
        if !sheet.exists() {
            eprintln!("skipping: spritesheet not present");
            return;
        }
        let interval = frame_interval_ms("idle", &sheet).expect("interval");
        let frames = load_frames("idle", 1.0, &sheet).expect("load");
        let expected = (LOOP_MS as f64) / (frames.len().max(1) as f64);
        assert!((interval - expected).abs() < 1e-9);
    }

    #[test]
    fn blank_detection_on_empty_image() {
        let blank = RgbaImage::from_pixel(FRAME_W, FRAME_H, Rgba([0, 0, 0, 0]));
        assert!(is_blank(&blank));
        let mut visible = blank.clone();
        visible.get_pixel_mut(10, 10).0 = [255, 255, 255, 255];
        assert!(!is_blank(&visible));
    }
}

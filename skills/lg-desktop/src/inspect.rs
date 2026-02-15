use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const STATE_PATH: &str = "/shared/lg-state.json";
const SCREENSHOT_DIR: &str = "/shared/screenshots";

#[derive(Serialize, Deserialize)]
struct InspectOutput {
    timestamp: String,
    desktop_size: [u32; 2],
    focused_window: String,
    windows: Vec<WindowInfo>,
    changes_since_last: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct WindowInfo {
    id: String,
    title: String,
    geometry: Geometry,
    ocr_text: String,
    /// Clickable text elements with absolute desktop coordinates.
    /// Only populated for changed windows (when OCR runs).
    /// To click an element: use center point (x + w/2, y + h/2).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    elements: Vec<TextElement>,
    changed: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct Geometry {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

/// A text element found by OCR with its bounding box in absolute desktop coordinates.
///
/// Design decision: We use tesseract TSV output to get per-word bounding boxes.
/// Alternative considered: AT-SPI (accessibility API) would give semantic UI elements
/// (buttons, fields) with labels, but requires python3-gi + python3-atspi (~50MB)
/// and a Python runtime in the container. Tesseract is already installed and gives
/// text positions with zero additional dependencies. The trade-off is that we get
/// text positions rather than semantic widget types, but for click targeting this
/// is sufficient -- the AI can click on any visible text element by its coordinates.
///
/// Coordinates are absolute (window position + element offset within screenshot).
/// Known limitation: window decorations may cause ~30px y-offset since scrot -u
/// captures including title bar but wmctrl reports content area position. In practice
/// XFCE title bars are thin and most clickable elements are well within the content
/// area, so the offset rarely causes misclicks.
#[derive(Serialize, Deserialize)]
struct TextElement {
    text: String,
    /// Absolute desktop X coordinate (top-left of bounding box)
    x: i32,
    /// Absolute desktop Y coordinate (top-left of bounding box)
    y: i32,
    w: u32,
    h: u32,
    confidence: f32,
}

#[derive(Serialize, Deserialize, Default)]
struct PreviousState {
    windows: HashMap<String, String>, // id -> hash
}

fn get_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Format as ISO 8601 (approximate, no chrono dependency)
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    // Approximate date calculation from epoch
    let (year, month, day) = days_to_date(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_date(days_since_epoch: u64) -> (u64, u64, u64) {
    // Simplified date calculation from days since 1970-01-01
    let mut y = 1970;
    let mut remaining = days_since_epoch;
    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let days_in_months: [u64; 12] = if is_leap_year(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0;
    for &dim in &days_in_months {
        if remaining < dim {
            break;
        }
        remaining -= dim;
        m += 1;
    }
    (y, m + 1, remaining + 1)
}

fn is_leap_year(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn get_desktop_size() -> [u32; 2] {
    let output = Command::new("xprop")
        .args(["-root", "_NET_DESKTOP_GEOMETRY"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            // Format: "_NET_DESKTOP_GEOMETRY(CARDINAL) = 1920, 1080"
            if let Some(eq_pos) = text.find('=') {
                let values: Vec<u32> = text[eq_pos + 1..]
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if values.len() == 2 {
                    return [values[0], values[1]];
                }
            }
            eprintln!("[lg-inspect] warning: could not parse desktop size from xprop output");
            [0, 0]
        }
        Ok(out) => {
            eprintln!(
                "[lg-inspect] warning: xprop failed (exit {}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
            [0, 0]
        }
        Err(e) => {
            eprintln!("[lg-inspect] warning: failed to run xprop: {e}");
            [0, 0]
        }
    }
}

fn get_focused_window() -> String {
    let output = Command::new("xdotool").arg("getactivewindow").output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Ok(out) => {
            eprintln!(
                "[lg-inspect] warning: xdotool getactivewindow failed (exit {})",
                out.status
            );
            String::new()
        }
        Err(e) => {
            eprintln!("[lg-inspect] warning: failed to run xdotool: {e}");
            String::new()
        }
    }
}

fn get_window_list() -> Vec<(String, Geometry, String)> {
    let output = Command::new("wmctrl").args(["-lG"]).output();

    let out = match output {
        Ok(out) if out.status.success() => out,
        Ok(out) => {
            eprintln!(
                "[lg-inspect] warning: wmctrl failed (exit {}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
            return Vec::new();
        }
        Err(e) => {
            eprintln!("[lg-inspect] warning: failed to run wmctrl: {e}");
            return Vec::new();
        }
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let mut windows = Vec::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // wmctrl -lG format: ID DESKTOP X Y W H HOSTNAME TITLE...
        if parts.len() >= 8 {
            let id = parts[0].to_string();
            let x = parts[2].parse().unwrap_or(0);
            let y = parts[3].parse().unwrap_or(0);
            let w = parts[4].parse().unwrap_or(0);
            let h = parts[5].parse().unwrap_or(0);
            let title = parts[7..].join(" ");
            let geom = Geometry { x, y, w, h };
            windows.push((id, geom, title));
        }
    }

    windows
}

fn capture_window(window_id: &str) -> Option<String> {
    if let Err(e) = fs::create_dir_all(SCREENSHOT_DIR) {
        eprintln!("[lg-inspect] warning: failed to create screenshot dir: {e}");
        return None;
    }
    let path = format!("{SCREENSHOT_DIR}/{window_id}.png");

    // Focus the window, then capture the focused window with scrot -u
    let focus = Command::new("xdotool")
        .args(["windowfocus", "--sync", window_id])
        .status();
    if !matches!(focus, Ok(s) if s.success()) {
        eprintln!("[lg-inspect] warning: failed to focus window {window_id}");
        return None;
    }

    let status = Command::new("scrot")
        .args(["-u", "-z", "-o", &path])
        .status();

    match status {
        Ok(s) if s.success() => Some(path),
        Ok(s) => {
            eprintln!(
                "[lg-inspect] warning: scrot exited with {} for window {window_id}",
                s.code().map_or("signal".to_string(), |c| c.to_string())
            );
            None
        }
        Err(e) => {
            eprintln!("[lg-inspect] error: failed to execute scrot: {e}");
            None
        }
    }
}

/// Run OCR and extract both full text and per-word bounding boxes.
///
/// Uses `tesseract tsv` output format which gives word-level positions.
/// We convert element coordinates to absolute desktop coordinates by adding
/// the window's geometry offset, so the AI can directly click on elements.
///
/// Design decision: confidence threshold is 40%. Lower catches more text but
/// adds noise tokens. Higher misses faint/small text. 40% was chosen as a
/// balance after testing with XFCE default theme -- most real UI text scores
/// >80%, while noise/artifacts score <30%.
fn ocr_image_with_elements(image_path: &str, window_geom: &Geometry) -> (String, Vec<TextElement>) {
    let output = Command::new("tesseract")
        .args([image_path, "stdout", "-l", "eng+jpn", "tsv"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut elements = Vec::new();
            let mut lines: Vec<(u32, Vec<String>)> = Vec::new();
            let mut current_line: u32 = 0;
            let mut current_words: Vec<String> = Vec::new();

            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() < 12 {
                    continue;
                }
                // TSV level 5 = word
                let level: u32 = parts[0].parse().unwrap_or(0);
                if level != 5 {
                    continue;
                }
                let line_num: u32 = parts[4].parse().unwrap_or(0);
                let conf: f32 = parts[10].parse().unwrap_or(-1.0);
                let word = parts[11].trim();

                if word.is_empty() || conf < 40.0 {
                    continue;
                }

                let left: i32 = parts[6].parse().unwrap_or(0);
                let top: i32 = parts[7].parse().unwrap_or(0);
                let width: u32 = parts[8].parse().unwrap_or(0);
                let height: u32 = parts[9].parse().unwrap_or(0);

                elements.push(TextElement {
                    text: word.to_string(),
                    x: window_geom.x + left,
                    y: window_geom.y + top,
                    w: width,
                    h: height,
                    confidence: conf,
                });

                // Reconstruct text grouped by line
                if line_num != current_line && !current_words.is_empty() {
                    lines.push((current_line, std::mem::take(&mut current_words)));
                    current_line = line_num;
                } else if current_words.is_empty() {
                    current_line = line_num;
                }
                current_words.push(word.to_string());
            }

            if !current_words.is_empty() {
                lines.push((current_line, current_words));
            }

            let full_text = lines
                .iter()
                .map(|(_, words)| words.join(" "))
                .collect::<Vec<_>>()
                .join("\n");

            (full_text, elements)
        }
        Ok(out) => {
            eprintln!(
                "[lg-inspect] warning: tesseract failed (exit {}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
            (String::new(), Vec::new())
        }
        Err(e) => {
            eprintln!("[lg-inspect] warning: failed to run tesseract: {e}");
            (String::new(), Vec::new())
        }
    }
}

fn compute_hash(path: &str) -> String {
    match fs::read(path) {
        Ok(data) => {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        }
        Err(e) => {
            eprintln!("[lg-inspect] warning: failed to read {path} for hashing: {e}");
            String::new()
        }
    }
}

fn load_previous_state() -> PreviousState {
    match fs::read_to_string(STATE_PATH) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(state) => state,
            Err(e) => {
                eprintln!("[lg-inspect] warning: corrupt state file {STATE_PATH}, resetting: {e}");
                PreviousState::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => PreviousState::default(),
        Err(e) => {
            eprintln!("[lg-inspect] warning: failed to read state file {STATE_PATH}: {e}");
            PreviousState::default()
        }
    }
}

fn save_state(state: &PreviousState) -> Result<()> {
    if let Some(parent) = std::path::Path::new(STATE_PATH).parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    let tmp_path = format!("{STATE_PATH}.tmp");
    fs::write(&tmp_path, &json).context("failed to write temp state")?;
    fs::rename(&tmp_path, STATE_PATH).context("failed to rename temp state to final")?;
    Ok(())
}

fn main() -> Result<()> {
    // --changes-only: only include changed windows in output (reduces token overhead)
    let changes_only = std::env::args().any(|a| a == "--changes-only");

    let timestamp = get_timestamp();
    let desktop_size = get_desktop_size();
    let focused_window = get_focused_window();
    let window_list = get_window_list();
    let previous = load_previous_state();

    let mut new_state = PreviousState::default();
    let mut windows = Vec::new();
    let mut changes = Vec::new();

    for (id, geometry, title) in &window_list {
        let screenshot_path = capture_window(id);
        let (is_changed, ocr_text, elements) = match &screenshot_path {
            Some(path) => {
                let h = compute_hash(path);
                let prev_hash = previous.windows.get(id).map(String::as_str);
                let did_change = prev_hash != Some(&h);
                let (ocr, elems) = if did_change {
                    ocr_image_with_elements(path, geometry)
                } else {
                    (String::new(), Vec::new())
                };
                new_state.windows.insert(id.clone(), h);
                (did_change, ocr, elems)
            }
            None => (true, String::new(), Vec::new()),
        };

        if is_changed {
            changes.push(id.clone());
        }

        windows.push(WindowInfo {
            id: id.clone(),
            title: title.clone(),
            geometry: *geometry,
            ocr_text,
            elements,
            changed: is_changed,
        });
    }

    // Restore original focus after capturing all windows
    if !focused_window.is_empty() {
        let _ = Command::new("xdotool")
            .args(["windowfocus", "--sync", &focused_window])
            .status();
    }

    if let Err(e) = save_state(&new_state) {
        eprintln!("[lg-inspect] warning: failed to save state: {e}");
    }

    // Filter to changed windows only when --changes-only is set.
    // This reduces JSON output significantly when only verifying an action result.
    let filtered_windows = if changes_only {
        windows.into_iter().filter(|w| w.changed).collect()
    } else {
        windows
    };

    let output = InspectOutput {
        timestamp,
        desktop_size,
        focused_window,
        windows: filtered_windows,
        changes_since_last: changes,
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_to_date_epoch() {
        assert_eq!(days_to_date(0), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_date_known_date() {
        // 2000-01-01 = 10957 days since epoch
        assert_eq!(days_to_date(10957), (2000, 1, 1));
    }

    #[test]
    fn test_days_to_date_leap_day() {
        // 2024-02-29 = 19782 days since epoch
        assert_eq!(days_to_date(19782), (2024, 2, 29));
    }

    #[test]
    fn test_days_to_date_end_of_year() {
        // 2023-12-31 = 19722 days since epoch
        assert_eq!(days_to_date(19722), (2023, 12, 31));
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000)); // divisible by 400
        assert!(!is_leap_year(1900)); // divisible by 100 but not 400
        assert!(is_leap_year(2024)); // divisible by 4
        assert!(!is_leap_year(2023)); // not divisible by 4
    }

    #[test]
    fn test_get_timestamp_format() {
        let ts = get_timestamp();
        // Should match ISO 8601 pattern: YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], "Z");
    }

    #[test]
    fn test_geometry_copy() {
        let g = Geometry { x: 10, y: 20, w: 100, h: 200 };
        let g2 = g; // Copy
        assert_eq!(g.x, g2.x);
        assert_eq!(g.w, g2.w);
    }
}

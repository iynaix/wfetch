// implementation adapted from:
// https://github.com/Canop/terminal-light/blob/main/src/xterm.rs

use crate::colors::Rgba8;
use color_eyre::eyre::Result;

fn query_xterm(query: &str, timeout_ms: u16) -> Result<String> {
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled};
    let switch_to_raw = !is_raw_mode_enabled()?;
    if switch_to_raw {
        enable_raw_mode()?;
    }
    let res = xterm_query::query(query, timeout_ms)?;
    if switch_to_raw {
        disable_raw_mode()?;
    }
    Ok(res)
}

/// Query the bg color, assuming the terminal is in raw mode,
/// using the "dynamic colors" OSC escape sequence.
pub fn query_term_color(color: u8) -> Option<Rgba8> {
    // we use the "dynamic colors" OSC escape sequence. It's sent with a ? for
    // a query and normally answered by the terminal with a color.
    // References:
    // - https://stackoverflow.com/a/28334701/263525
    // - https://invisible-island.net/xterm/ctlseqs/ctlseqs.html

    // The string we receive is like `"\u{1b}]11;rgb:<red>/<green>/<blue>\u{1b}\\"`
    // where `<red>`, `<green>`, and `<blue>` are 4 hex digits.
    // Most terminals don't support such precision so they fill the 4 digits
    // by repeating their 2 digits precision.
    // For example, supposing the background is in #38A4C9 (blue),
    // then we receive `"\u{1b}]11;rgb:3838/a4a4/c9c9\u{1b}\\"`.
    // We read only the most significant hex digits which are good enough
    // in all cases.
    let s = query_xterm(&format!("\x1b]4;{color};?\x07"), 100).ok()?;
    let raw_color = s.strip_prefix(&format!("\x1b]4;{color};rgb:"))?;

    if raw_color.len() >= 14 {
        Some(image::Rgba([
            u8::from_str_radix(&raw_color[0..2], 16).ok()?,
            u8::from_str_radix(&raw_color[5..7], 16).ok()?,
            u8::from_str_radix(&raw_color[10..12], 16).ok()?,
            255,
        ]))
    } else {
        None
    }
}

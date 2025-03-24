use std::collections::HashMap;

use image::Rgba;

use crate::{WFetchResult, full_path};

fn normalize_channel(channel: u8) -> f64 {
    let channel = f64::from(channel) / 255.0;
    if channel <= 0.03928 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

pub type Rgba8 = Rgba<u8>;
pub const BLACK: Rgba8 = Rgba([0, 0, 0, 255]);

pub trait Rgba8Ext {
    type Err;

    fn from_str(s: &str) -> Result<Self, Self::Err>
    where
        Self: Sized;

    #[must_use]
    fn with_alpha(self, alpha: u8) -> Self;

    fn distance(&self, other: Self) -> f64;

    #[must_use]
    fn multiply(&self, other: Self) -> Self;

    fn relative_luminance(&self) -> f64;

    fn contrast_ratio(&self, other: &Self) -> f64;

    /// ansi color code for terminal background in a format suitable for fastfetch
    fn term_fg(&self) -> String;

    /// ansi color code for terminal background in a format suitable for fastfetch
    fn term_bg(&self) -> String;
}

impl Rgba8Ext for Rgba8 {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_start_matches('#');

        let r = u8::from_str_radix(&s[0..2], 16)?;
        let g = u8::from_str_radix(&s[2..4], 16)?;
        let b = u8::from_str_radix(&s[4..6], 16)?;

        let alpha = if s.len() == 8 {
            u8::from_str_radix(&s[7..8], 16)?
        } else {
            255
        };

        Ok(Self([r, g, b, alpha]))
    }

    fn distance(&self, other: Self) -> f64 {
        let dr = f64::from(self[0]) - f64::from(other[0]);
        let dg = f64::from(self[1]) - f64::from(other[1]);
        let db = f64::from(self[2]) - f64::from(other[2]);
        db.mul_add(db, dr.mul_add(dr, dg * dg)).sqrt()
    }

    fn with_alpha(self, alpha: u8) -> Self {
        Self([self[0], self[1], self[2], alpha])
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn multiply(&self, other: Self) -> Self {
        Self([
            (f64::from(self[0]) * f64::from(other[0]) / 255.0) as u8,
            (f64::from(self[1]) * f64::from(other[1]) / 255.0) as u8,
            (f64::from(self[2]) * f64::from(other[2]) / 255.0) as u8,
            self[3],
        ])
    }

    /// relative luminance, as defined by WCAG
    /// <https://www.w3.org/TR/WCAG20/#relativeluminancedef>
    fn relative_luminance(&self) -> f64 {
        let r = normalize_channel(self[0]);
        let g = normalize_channel(self[1]);
        let b = normalize_channel(self[2]);

        0.0722_f64.mul_add(b, 0.2126_f64.mul_add(r, 0.7152 * g))
    }

    fn contrast_ratio(&self, other: &Self) -> f64 {
        let l1 = self.relative_luminance();
        let l2 = other.relative_luminance();

        if l1 > l2 {
            (l1 + 0.05) / (l2 + 0.05)
        } else {
            (l2 + 0.05) / (l1 + 0.05)
        }
    }

    fn term_fg(&self) -> String {
        format!("38;2;{};{};{}", self[0], self[1], self[2])
    }

    fn term_bg(&self) -> String {
        format!("48;2;{};{};{}", self[0], self[1], self[2])
    }
}

fn color_pair_score(color1: Rgba8, color2: Rgba8) -> f64 {
    const WCAG_THRESHOLD: f64 = 3.0;

    // must meet minimum contrast threshold (WCAG AA for UI components)
    let color1_color2 = color1.contrast_ratio(&color2);
    if color1_color2 < WCAG_THRESHOLD {
        return 0.0;
    }

    let color1_black = color1.contrast_ratio(&BLACK);
    let color2_black = color2.contrast_ratio(&BLACK);

    // at least one color should be readable on dark background
    if color1_black < WCAG_THRESHOLD && color2_black < WCAG_THRESHOLD {
        return 0.0;
    }

    color1_black + color2_black
}

/// find the most contrasting pair of colors in a list
pub fn most_contrasting_pair(colors: &[Rgba8]) -> (Rgba8, Rgba8) {
    let mut max_score = 0.0;
    let mut most_contrasting_pair = (image::Rgba([0, 0, 0, 255]), image::Rgba([0, 0, 0, 255]));

    for color1 in colors {
        for color2 in colors {
            if color1 == color2 {
                continue;
            }

            let score = color_pair_score(*color1, *color2);
            if score > max_score {
                max_score = score;
                most_contrasting_pair = (*color1, *color2);
            }
        }
    }
    most_contrasting_pair
}

#[derive(serde::Deserialize)]
struct NixInfo {
    colors: HashMap<String, String>,
}

fn term_colors_from_json() -> WFetchResult<Vec<Rgba8>> {
    let contents = std::fs::read_to_string(full_path("~/.cache/wallust/nix.json"))?;

    let colors = serde_json::from_str::<NixInfo>(&contents)?.colors;

    (0..16)
        .map(|i| {
            let color_str = colors
                .get(&format!("color{i}"))
                .ok_or("failed to get color")?;
            let color = Rgba::from_str(color_str)?;
            Ok(color)
        })
        .collect()
}

fn term_colors_from_xterm() -> WFetchResult<Vec<Rgba8>> {
    (0..16).map(crate::xterm::query_term_color).collect()
}

#[allow(clippy::module_name_repetitions)]
pub fn get_term_colors() -> WFetchResult<Vec<Rgba8>> {
    term_colors_from_json().or_else(|_| term_colors_from_xterm())
}

use std::collections::HashMap;

use image::Rgba;

use crate::{full_path, WFetchResult};

pub const TERMINAL_COLORS: [&str; 16] = [
    "black",
    "blue",
    "green",
    "cyan",
    "red",
    "magenta",
    "yellow",
    "white",
    "bright_black",
    "bright_blue",
    "bright_green",
    "bright_cyan",
    "bright_red",
    "bright_magenta",
    "bright_yellow",
    "bright_white",
];

pub type Rgba8 = Rgba<u8>;

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
}

/// find the most contrasting pair of colors in a list
pub fn most_contrasting_pair(colors: &[Rgba8]) -> (Rgba8, Rgba8) {
    let mut max_distance = 0.0;
    let mut most_contrasting_pair = (image::Rgba([0, 0, 0, 255]), image::Rgba([0, 0, 0, 255]));

    for color1 in colors {
        for color2 in colors {
            if color1 == color2 {
                continue;
            }

            let distance = color1.distance(*color2);
            if distance > max_distance {
                max_distance = distance;
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

fn logo_colors_from_json() -> WFetchResult<Vec<Rgba8>> {
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

fn logo_colors_from_xterm() -> WFetchResult<Vec<Rgba8>> {
    (0..16).map(crate::xterm::query_term_color).collect()
}

#[allow(clippy::module_name_repetitions)]
pub fn get_logo_colors() -> WFetchResult<Vec<Rgba8>> {
    logo_colors_from_json().or_else(|_| logo_colors_from_xterm())
}

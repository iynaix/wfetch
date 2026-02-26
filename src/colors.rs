use std::collections::HashMap;

use image::{Pixel, Rgba};
use palette::{IntoColor, Lab, Srgb, color_difference::EuclideanDistance};

use crate::{WFetchResult, full_path};

pub type Rgba8 = Rgba<u8>;
pub const BLACK: Rgba8 = Rgba([0, 0, 0, 255]);
pub const WHITE: Rgba8 = Rgba([255, 255, 255, 255]);

pub trait Rgba8Ext {
    type Err;

    fn from_str(s: &str) -> Result<Self, Self::Err>
    where
        Self: Sized;

    fn to_lab(&self) -> Lab;

    #[must_use]
    fn with_alpha(self, alpha: u8) -> Self;

    fn distance(&self, other: Self) -> f64;

    #[must_use]
    fn multiply(&self, other: Self) -> Self;

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

    fn to_lab(&self) -> Lab {
        Srgb::new(self[0], self[1], self[2])
            .into_format::<f32>()
            .into_color()
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn multiply(&self, other: Self) -> Self {
        self.map2(&other, |a, b| (f64::from(a) * f64::from(b) / 255.0) as u8)
    }

    fn term_fg(&self) -> String {
        format!("38;2;{};{};{}", self[0], self[1], self[2])
    }

    fn term_bg(&self) -> String {
        format!("48;2;{};{};{}", self[0], self[1], self[2])
    }
}

/// find the most contrasting n colors in a list
pub fn most_contrasting_colors(colors: &[Rgba<u8>], n: usize) -> Vec<Rgba<u8>> {
    let colors: HashMap<_, _> = colors.iter().map(|c| (c, c.to_lab())).collect();
    let mut unique_colors: Vec<Lab> = Vec::new();

    for lab in colors.values() {
        // Only keep the color if it's far enough from all current unique colors
        // deltaE 2.3 is barely perceptible, 10.0 is significantly different
        if unique_colors.iter().all(|c| lab.distance(*c) > 10.0) {
            unique_colors.push(*lab);
        }
    }

    let mut max_score = 0.0;
    let mut pair = (Lab::default(), Lab::default());

    for c1 in &unique_colors {
        for c2 in &unique_colors {
            if c1 == c2 {
                continue;
            }

            // lab distance
            let score = c1.distance(*c2);
            if score > max_score {
                max_score = score;
                pair = (*c1, *c2);
            }
        }
    }

    let mut selected = vec![pair.0, pair.1];

    for _ in 2..n {
        let min = unique_colors
            .iter()
            .min_by(|a, b| {
                let a_dist: f32 = selected
                    .iter()
                    .filter(|sel| sel != a && sel != b)
                    .map(|sel| a.distance(*sel))
                    .sum();
                let b_dist: f32 = selected
                    .iter()
                    .filter(|sel| sel != a && sel != b)
                    .map(|sel| b.distance(*sel))
                    .sum();

                a_dist.total_cmp(&b_dist)
            })
            .expect("no min");

        selected.push(*min);
    }

    selected
        .iter()
        .map(|sel| {
            **colors
                .iter()
                .find(|(_, l)| *l == sel)
                .expect("could not find lab color equivalent")
                .0
        })
        .collect()
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

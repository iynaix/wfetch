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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Color(u8, u8, u8);

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl std::str::FromStr for Color {
    type Err = std::num::ParseIntError;

    // Function to parse a color string in #RRGGBB format into RGB components
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = u8::from_str_radix(&s[1..3], 16)?;
        let g = u8::from_str_radix(&s[3..5], 16)?;
        let b = u8::from_str_radix(&s[5..7], 16)?;

        Ok(Self(r, g, b))
    }
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b)
    }

    fn color_distance(&self, color2: &Self) -> f64 {
        let dr = f64::from(i16::from(self.0) - i16::from(color2.0));
        let dg = f64::from(i16::from(self.1) - i16::from(color2.1));
        let db = f64::from(i16::from(self.2) - i16::from(color2.2));
        db.mul_add(db, dr.mul_add(dr, dg * dg)).sqrt()
    }

    pub fn imagemagick_replace_args(&self, opaque: &str) -> Vec<String> {
        [
            "-fuzz",
            "10%",
            "-fill",
            &self.to_string(),
            "-opaque",
            &opaque,
        ]
        .iter()
        .map(std::string::ToString::to_string)
        .collect()
    }
}

/// find the most contrasting pair of colors in a list
pub fn most_contrasting_pair(colors: &[Color]) -> (Color, Color) {
    let mut max_distance = 0.0;
    let mut most_contrasting_pair = (Color::default(), Color::default());

    for color1 in colors {
        for color2 in colors {
            if color1 == color2 {
                continue;
            }

            let distance = color1.color_distance(color2);
            if distance > max_distance {
                max_distance = distance;
                most_contrasting_pair = (color1.clone(), color2.clone());
            }
        }
    }

    most_contrasting_pair
}

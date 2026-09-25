use std::{
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
};

use color_eyre::eyre::{OptionExt, Result};
use fast_image_resize::images::Image;
use fast_image_resize::{IntoImageView, PixelType, ResizeOptions, Resizer};
use image::{ImageBuffer, ImageEncoder, ImageReader, Rgba, codecs::png::PngEncoder};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::{
    cli::WFetchArgs,
    colors::{Rgba8, Rgba8Ext, most_contrasting_colors},
    create_output_file, get_terminal_cell_height, wallpaper,
};
use crate::{colors::get_term_colors, wallpaper::geom_from_str};

const NIX_COLOR1: [u8; 4] = [0x7e, 0xba, 0xe4, 255];
const NIX_COLOR2: [u8; 4] = [0x52, 0x77, 0xc3, 255];

fn get_hyprland_scale() -> Option<f64> {
    #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct HyprMonitor {
        pub scale: f64,
        pub focused: bool,
    }

    // no scale arg provided, try getting it from hyprland
    Command::new("hyprctl")
        .arg("monitors")
        .arg("-j")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|stdout| serde_json::from_str::<Vec<HyprMonitor>>(&stdout).ok())
        .and_then(|monitors| monitors.into_iter().find(|m| m.focused))
        .map(|monitor| monitor.scale)
}

fn get_niri_scale() -> Option<f64> {
    #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
    pub struct NiriMonitor {
        pub logical: Option<NiriLogical>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
    pub struct NiriLogical {
        pub scale: f64,
    }

    Command::new("niri")
        .arg("msg")
        .arg("--json")
        .arg("focused-output")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|stdout| serde_json::from_str::<NiriMonitor>(&stdout).ok())
        .and_then(|monitor| monitor.logical.map(|logical| logical.scale))
}

/// returns new sizes adjusted for the given scale
fn resize_with_scale(scale: Option<f64>, width: u32, height: u32, term: &str) -> (u32, u32) {
    // no scale arg, provided, try getting scale from hyprland or niri
    let mut scale = scale
        .or_else(get_hyprland_scale)
        .or_else(get_niri_scale)
        .unwrap_or(1.0);

    if term == "ghostty" || term.contains("wezterm") {
        scale = scale.ceil();
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    (
        (f64::from(width) * scale).floor() as u32,
        (f64::from(height) * scale).floor() as u32,
    )
}

pub fn image_from_arg(arg: &str) -> Option<String> {
    if arg == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).ok()?;

        // valid image, write stdin to a file
        if let Ok(format) = image::guess_format(&buf) {
            // need to write the extension or Image has problems guessing the format later
            let ext = format.extensions_str()[0];
            let output = create_output_file(&format!("wfetch_stdin.{ext}")).ok()?;
            std::fs::write(&output, &buf).ok()?;
            return Some(output.to_string_lossy().to_string());
        }

        String::from_utf8(buf).ok().and_then(|s| {
            let full_path = std::fs::canonicalize(s.trim())
                .map(|p| p.to_string_lossy().to_string())
                .ok();

            wallpaper::detect(&full_path)
        })
    } else {
        wallpaper::detect(&Some(arg))
    }
}

/// calculates a target image height, taking cell height into account if possible for the perfect fit
fn image_height(target_height: u32, lines: u32) -> u32 {
    // kitty adds an extra blank row if the pixel height doesn't divide evenly
    // into the cell height
    // add 1 to image if it is a multiple to get the text to be vertically centered against
    // the image
    get_terminal_cell_height().map_or(target_height, |cell_height| {
        (cell_height * lines as f32).ceil() as u32 + 1
    })
}

/// creates the wallpaper image that fastfetch will display
pub fn resize_wallpaper(
    args: &WFetchArgs,
    term: &str,
    image_arg: &Option<String>,
) -> Result<PathBuf> {
    let output = create_output_file("wfetch.png")?;

    let wall = image_arg
        .as_ref()
        .and_then(|img| image_from_arg(img.as_str()))
        .unwrap_or_else(|| {
            eprintln!("Error: could not detect wallpaper!");
            std::process::exit(1);
        });

    ImageReader::open(&wall)?.decode()?;

    let mut fallback_geometry = {
        let (width, height) = image::image_dimensions(&wall)?;
        let (width, height) = (f64::from(width), f64::from(height));

        // get basic square crop in the center
        if width > height {
            (height, height, (width - height) / 2.0, 0.0)
        } else {
            (width, width, 0.0, (height - width) / 2.0)
        }
    };

    // use the crop argument if provided
    if let Some(crop) = args.crop.as_ref() {
        fallback_geometry = geom_from_str(crop).unwrap_or(fallback_geometry);
    } else {
        fallback_geometry = wallpaper::info(&wall, fallback_geometry);
    }

    // force the crop to be square
    let (w, h, x, y) = fallback_geometry;
    fallback_geometry = (w.min(h), w.min(h), x, y);

    let img = ImageReader::open(&wall)?.decode()?;

    let dst_size = args.image_size.unwrap_or_else(|| {
        if args.challenge {
            image_height(350, 13 + 4)
        } else {
            image_height(270, 13)
        }
    });

    let (dst_size, _) = resize_with_scale(args.scale, dst_size, dst_size, term);

    #[allow(clippy::cast_sign_loss)]
    let mut dest = Image::new(
        dst_size,
        dst_size,
        img.pixel_type().ok_or_eyre("Unknown pixel type")?,
    );
    let (w, h, x, y) = fallback_geometry;
    Resizer::new().resize(&img, &mut dest, &ResizeOptions::new().crop(x, y, w, h))?;

    let mut result_buf = std::io::BufWriter::new(std::fs::File::create(&output)?);

    #[allow(clippy::cast_sign_loss)]
    PngEncoder::new(&mut result_buf).write_image(
        dest.buffer(),
        dst_size,
        dst_size,
        img.color().into(),
    )?;

    Ok(output)
}

fn save_png(
    src: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    size: (u32, u32),
    output: &PathBuf,
) -> Result<()> {
    let (mut dst_w, mut dst_h) = size;

    // resize src to fit within size
    let (src_w, src_h) = src.dimensions();

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    if src_w > src_h {
        dst_h = (f64::from(src_h) * f64::from(dst_w) / f64::from(src_w)) as u32;
    } else {
        dst_w = (f64::from(src_w) * f64::from(dst_h) / f64::from(src_h)) as u32;
    }

    let src_view = Image::from_vec_u8(src.width(), src.height(), src.into_raw(), PixelType::U8x4)?;

    #[allow(clippy::cast_sign_loss)]
    let mut dest = Image::new(dst_w, dst_h, PixelType::U8x4);
    Resizer::new().resize(&src_view, &mut dest, None)?;

    let mut result_buf = std::io::BufWriter::new(std::fs::File::create(output)?);

    #[allow(clippy::cast_sign_loss)]
    PngEncoder::new(&mut result_buf).write_image(
        dest.buffer(),
        dst_w,
        dst_h,
        image::ColorType::Rgba8.into(),
    )?;

    Ok(())
}

pub struct Logo {
    args: WFetchArgs,
    nixos: bool,
    term: String,
    tmux: bool,
}

impl Logo {
    pub fn new(args: &WFetchArgs, nixos: bool, term: &str, tmux: bool) -> Self {
        Self {
            args: args.clone(),
            nixos,
            term: term.to_string(),
            tmux,
        }
    }

    fn with_backend<S>(&self, source: S) -> JsonValue
    where
        S: AsRef<str> + Serialize,
    {
        let logo_backend = if self.term == "konsole" {
            "iterm"
        } else if self.term == "foot" {
            "sixel"
        } else if self.tmux {
            "kitty-icat"
        } else {
            "kitty"
        };

        json!({
            "type": logo_backend,
            "source": source,
            "recache": true,
            "preserveAspectRatio": true,
        })
    }

    pub fn waifu1(&self, color1: &Rgba8, color2: &Rgba8) -> Result<JsonValue> {
        let output = create_output_file("wfetch.png")?;

        let replace1 = Rgba8::from(NIX_COLOR1);
        let replace2 = Rgba8::from(NIX_COLOR2);

        let mut src = image::load_from_memory(include_bytes!("../assets/nixos1.png"))?.into_rgba8();

        let fuzz = 0.1 * (255.0_f64 * 255.0_f64 * 3.0_f64).sqrt();

        for pixel in src.pixels_mut() {
            if pixel.distance(replace1) < fuzz {
                *pixel = color1.with_alpha(pixel[3]);
            } else if pixel.distance(replace2) < fuzz {
                *pixel = color2.with_alpha(pixel[3]);
            }
        }

        let side = self
            .args
            .image_size
            .unwrap_or(if self.args.challenge { 380 } else { 300 });

        save_png(
            src,
            resize_with_scale(self.args.scale, side, side, &self.term),
            &output,
        )?;

        Ok(self.with_backend(output.to_string_lossy()))
    }

    pub fn waifu2(&self, color1: &Rgba8, color2: &Rgba8) -> Result<JsonValue> {
        let output = create_output_file("wfetch.png")?;

        let mut src = image::load_from_memory(include_bytes!("../assets/nixos2.png"))?.into_rgba8();

        let mask1 =
            image::load_from_memory(include_bytes!("../assets/nixos2-mask1.jpg"))?.into_rgba8();

        let mask2 =
            image::load_from_memory(include_bytes!("../assets/nixos2-mask2.jpg"))?.into_rgba8();

        let fuzz = 0.1 * (255.0_f64 * 255.0_f64 * 3.0_f64).sqrt();
        let black = Rgba([0, 0, 0, 255]);

        for (x, y, pixel) in src.enumerate_pixels_mut() {
            if black.distance(pixel.multiply(*mask1.get_pixel(x, y))) < fuzz {
                *pixel = color1.with_alpha(pixel[3]);
            }

            if black.distance(pixel.multiply(*mask2.get_pixel(x, y))) < fuzz {
                *pixel = color2.with_alpha(pixel[3]);
            }
        }

        let side = self.args.image_size.unwrap_or_else(|| {
            if self.args.challenge {
                image_height(350, 13 + 4)
            } else {
                image_height(270, 11)
            }
        });

        save_png(
            src,
            resize_with_scale(self.args.scale, side, side, &self.term),
            &output,
        )?;

        Ok(self.with_backend(output.to_string_lossy()))
    }

    /// creates the wallpaper ascii that fastfetch will display
    pub fn show_wallpaper_ascii(&self, image_arg: &Option<String>) -> Result<PathBuf> {
        let img = resize_wallpaper(&self.args, &self.term, image_arg)?;
        let output_dir = img.parent().ok_or_eyre("could not get output dir")?;

        // NOTE: uses patched version of ascii-image-converter to be able to output colored ascii text to file
        Command::new("ascii-image-converter")
            .arg("--color")
            .arg("--braille")
            .arg("--threshold")
            .arg("50")
            .arg("--width")
            .arg(self.args.ascii_size.to_string())
            // do not output to terminal
            .arg("--only-save")
            .arg("--save-txt")
            // weird api: takes a directory, saves as wfetch-ascii-art.png
            .arg(output_dir)
            .arg(&img)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        Ok(output_dir.join("wfetch-ascii-art.txt"))
    }

    pub fn waifu1_default(&self) -> Result<JsonValue> {
        self.waifu1(&Rgba8::from(NIX_COLOR1), &Rgba8::from(NIX_COLOR2))
    }

    pub fn waifu2_default(&self) -> Result<JsonValue> {
        self.waifu2(&Rgba8::from(NIX_COLOR1), &Rgba8::from(NIX_COLOR2))
    }

    pub fn hollow_default(&self) -> JsonValue {
        json!({
            "data": include_str!("../assets/nixos_hollow.txt"),
            "color": json!({ "1": "blue", "2": "cyan" }),
        })
    }

    pub fn hollow_large_default(&self) -> JsonValue {
        json!({
            "source": "nixos2",
            "color": json!({ "1": "blue", "2": "cyan" }),
        })
    }

    pub fn smooth_default(&self) -> JsonValue {
        json!({
            "data": include_str!("../assets/nixos_smooth.txt"),
            "color": json!({
                "1": "38;5;4", // blue
                "2": "38;5;6", // cyan
                "2": "48;5;6", // blue bg
                "2": "48;5;4", // cyan bg
             }),
        })
    }

    pub fn dots_default(&self) -> JsonValue {
        json!({
            "data": include_str!("../assets/nixos_dots.txt"),
            "color": json!({ "1": "blue", "2": "cyan" }),
        })
    }

    pub fn hashes_default(&self) -> JsonValue {
        json!({
            "source": "nixos_old",
            "color": json!({ "1": "blue", "2": "cyan" }),
        })
    }

    pub fn filled_default(&self) -> JsonValue {
        json!({
            "source": "nixos",
            "color": json!({
                "1": "38;5;4", // blue
                "2": "38;5;6", // cyan
                "2": "48;5;6", // blue bg
                "2": "48;5;4", // cyan bg
             }),
        })
    }

    pub fn module_for_tmux(&self) -> Result<JsonValue> {
        #[cfg(feature = "nixos")]
        if self.args.waifu {
            return self.waifu1_default();
        }
        #[cfg(feature = "nixos")]
        if self.args.waifu2 {
            return self.waifu2_default();
        }

        #[cfg(feature = "nixos")]
        if self.args.hollow {
            return Ok(self.hollow_default());
        }

        #[cfg(feature = "nixos")]
        if self.args.smooth {
            return Ok(self.smooth_default());
        }

        #[cfg(feature = "nixos")]
        if self.args.dots {
            return Ok(self.dots_default());
        }

        if self.nixos {
            return Ok(self.filled_default());
        }

        // use fastfetch default
        Ok(json!({ "source": null }))
    }

    #[allow(clippy::too_many_lines)]
    pub fn module(&self) -> Result<JsonValue> {
        if self.args.wallpaper_ascii.is_some() {
            return self
                .show_wallpaper_ascii(&self.args.wallpaper_ascii)
                .map(|ascii_file| {
                    json!({
                        "type": "auto",
                        "source": ascii_file,
                    })
                });
        }

        if self.args.wallpaper.is_some() {
            return resize_wallpaper(&self.args, &self.term, &self.args.wallpaper)
                .map(|wall| self.with_backend(wall.to_string_lossy()));
        }

        // handle tmux separately as the raw xterm sequences breaks rendering and text input
        if self.tmux {
            return self.module_for_tmux();
        }

        let term_colors = get_term_colors();
        if term_colors.is_empty() {
            #[cfg(feature = "nixos")]
            {
                if self.args.waifu {
                    return self.waifu1_default();
                }

                if self.args.waifu2 {
                    return self.waifu2_default();
                }

                if self.args.hollow {
                    return Ok(self.hollow_default());
                }

                if self.args.hollow {
                    return Ok(self.hollow_large_default());
                }

                if self.args.smooth {
                    return Ok(self.smooth_default());
                }

                if self.args.dots {
                    return Ok(self.dots_default());
                }

                if self.args.hashes {
                    return Ok(self.hashes_default());
                }

                if self.nixos {
                    return Ok(self.filled_default());
                }
            }
        } else {
            // remove background color to get contrast
            let contrasting_colors = most_contrasting_colors(&term_colors[1..], 2);
            let color1 = contrasting_colors[0];
            let color2 = contrasting_colors[1];

            #[cfg(feature = "nixos")]
            if self.args.waifu {
                return self.waifu1(&color1, &color2);
            }
            #[cfg(feature = "nixos")]
            if self.args.waifu2 {
                return self.waifu2(&color1, &color2);
            }

            #[cfg(feature = "nixos")]
            if self.args.hollow {
                return Ok(json!({
                    "data": include_str!("../assets/nixos_hollow.txt"),
                    "color": json!({
                        "1": color1.term_fg(),
                        "2": color2.term_fg(),
                    }),
                }));
            }

            #[cfg(feature = "nixos")]
            if self.args.hollow_large {
                return Ok(json!({
                    "source": "nixos2",
                    "color": json!({
                        "1": color1.term_fg(),
                        "2": color2.term_fg(),
                    }),
                }));
            }

            #[cfg(feature = "nixos")]
            if self.args.dots {
                return Ok(json!({
                    "data": include_str!("../assets/nixos_dots.txt"),
                    "color": json!({
                        "1": color1.term_fg(),
                        "2": color2.term_fg(),
                    }),
                }));
            }

            #[cfg(feature = "nixos")]
            if self.args.hashes {
                return Ok(json!({
                    "source": "nixos_old",
                    "color": json!({
                        "1": color1.term_fg(),
                        "2": color2.term_fg(),
                    }),
                }));
            }

            #[cfg(feature = "nixos")]
            if self.args.smooth {
                return Ok(json!({
                    "data": include_str!("../assets/nixos_smooth.txt"),
                    // note: it is 1 2 2 1 intentionally
                    "color": json!({
                        "1": color1.term_fg(),
                        "2": color2.term_fg(),
                        "3": color2.term_bg(),
                        "4": color1.term_bg(),
                    }),
                }));
            }

            if self.nixos {
                return Ok(json!({
                    "source": "nixos",
                    "color": json!({
                        "1": color1.term_fg(),
                        "2": color2.term_fg(),
                        "3": color1.term_fg(),
                        "4": color2.term_fg(),
                        "5": color1.term_fg(),
                        "6": color2.term_fg(),
                    }),
                }));
            }
        }

        // use fastfetch default
        Ok(json!({ "source": null }))
    }
}

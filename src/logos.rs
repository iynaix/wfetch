use std::{
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
};

use fast_image_resize::images::Image;
use fast_image_resize::{IntoImageView, PixelType, ResizeOptions, Resizer};
use image::{ImageBuffer, ImageEncoder, ImageReader, Rgba, codecs::png::PngEncoder};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::{
    asset_path,
    cli::WFetchArgs,
    colors::{self, Rgba8, Rgba8Ext},
    create_output_file, wallpaper,
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

pub fn image_from_arg(arg: &Option<String>) -> Option<String> {
    if *arg == Some("-".to_string()) {
        let mut buf = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buf)
            .expect("unable to read stdin");

        // valid image, write stdin to a file
        if let Ok(format) = image::guess_format(&buf) {
            // need to write the extension or Image has problems guessing the format later
            let ext = format.extensions_str()[0];
            let output = create_output_file(&format!("wfetch_stdin.{ext}"));
            std::fs::write(&output, &buf).expect("could not write stdin to file");
            return Some(output.to_string_lossy().to_string());
        }

        return String::from_utf8(buf).ok().and_then(|s| {
            let full_path = std::fs::canonicalize(s.trim())
                .map(|p| p.to_string_lossy().to_string())
                .ok();

            wallpaper::detect(&full_path)
        });
    }

    wallpaper::detect(arg)
}

/// creates the wallpaper image that fastfetch will display
pub fn resize_wallpaper(args: &WFetchArgs, term: &str, image_arg: &Option<String>) -> PathBuf {
    let output = create_output_file("wfetch.png");

    let img = image_from_arg(image_arg).unwrap_or_else(|| {
        eprintln!("Error: could not detect wallpaper!");
        std::process::exit(1);
    });

    let wall = wallpaper::detect(&Some(img)).unwrap_or_else(|| {
        eprintln!("Error: could not detect wallpaper!");
        std::process::exit(1);
    });

    ImageReader::open(&wall)
        .expect("could not open image")
        .decode()
        .expect("could not decode image");

    #[cfg_attr(not(feature = "iynaixos"), allow(unused_mut))]
    let mut fallback_geometry = {
        let (width, height) =
            image::image_dimensions(&wall).expect("could not get image dimensions");
        let (width, height) = (f64::from(width), f64::from(height));

        // get basic square crop in the center
        if width > height {
            (height, height, (width - height) / 2.0, 0.0)
        } else {
            (width, width, 0.0, (height - width) / 2.0)
        }
    };

    #[cfg(feature = "iynaixos")]
    {
        fallback_geometry = wallpaper::info(&wall, fallback_geometry);
    }

    // use the crop argument if provided
    if let Some(crop) = args.crop.as_ref() {
        fallback_geometry = geom_from_str(crop).unwrap_or(fallback_geometry);
    }

    // force the crop to be square
    let (w, h, x, y) = fallback_geometry;
    fallback_geometry = (w.min(h), w.min(h), x, y);

    let img = ImageReader::open(&wall)
        .expect("could not open image")
        .decode()
        .expect("could not decode image");

    let dst_size = args
        .image_size
        .unwrap_or(if args.challenge { 350 } else { 270 });

    let (dst_size, _) = resize_with_scale(args.scale, dst_size, dst_size, term);

    #[allow(clippy::cast_sign_loss)]
    let mut dest = Image::new(
        dst_size,
        dst_size,
        img.pixel_type().expect("could not get pixel type"),
    );
    let (w, h, x, y) = fallback_geometry;
    Resizer::new()
        .resize(&img, &mut dest, &ResizeOptions::new().crop(x, y, w, h))
        .expect("failed to resize image");

    let mut result_buf =
        std::io::BufWriter::new(std::fs::File::create(&output).expect("could not create file"));

    #[allow(clippy::cast_sign_loss)]
    PngEncoder::new(&mut result_buf)
        .write_image(dest.buffer(), dst_size, dst_size, img.color().into())
        .expect("failed to write wallpaper crop");

    output
}

fn save_png(src: ImageBuffer<image::Rgba<u8>, Vec<u8>>, size: (u32, u32), output: &PathBuf) {
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

    let src_view = Image::from_vec_u8(src.width(), src.height(), src.into_raw(), PixelType::U8x4)
        .expect("could not create image view");

    #[allow(clippy::cast_sign_loss)]
    let mut dest = Image::new(dst_w, dst_h, PixelType::U8x4);
    Resizer::new()
        .resize(&src_view, &mut dest, None)
        .expect("failed to resize image");

    let mut result_buf = std::io::BufWriter::new(
        std::fs::File::create(output)
            .unwrap_or_else(|_| panic!("could not create {}", output.display())),
    );

    #[allow(clippy::cast_sign_loss)]
    PngEncoder::new(&mut result_buf)
        .write_image(dest.buffer(), dst_w, dst_h, image::ColorType::Rgba8.into())
        .unwrap_or_else(|_| panic!("failed to write png for {}", output.display()));
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

    fn with_backend(&self, source: &str) -> JsonValue {
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
            "preserveAspectRatio": true,
        })
    }

    pub fn waifu1(&self, color1: &Rgba8, color2: &Rgba8) -> JsonValue {
        let output = create_output_file("wfetch.png");

        let replace1 = Rgba8::from(NIX_COLOR1);
        let replace2 = Rgba8::from(NIX_COLOR2);

        let mut src = ImageReader::open(asset_path("nixos1.png"))
            .expect("could not open nixos1.png")
            .decode()
            .expect("could not decode nixos1.png")
            .into_rgba8();

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
        );

        self.with_backend(
            output
                .to_str()
                .expect("could not convert output path to str"),
        )
    }

    pub fn waifu2(&self, color1: &Rgba8, color2: &Rgba8) -> JsonValue {
        let output = create_output_file("wfetch.png");

        let mut src = ImageReader::open(asset_path("nixos2.png"))
            .expect("could not open nixos2.png")
            .decode()
            .expect("could not decode nixos2.png")
            .into_rgba8();

        let mask1 = image::open(asset_path("nixos2-mask1.jpg"))
            .expect("could not open mask1")
            .to_rgba8();
        let mask2 = image::open(asset_path("nixos2-mask2.jpg"))
            .expect("could not open mask2")
            .to_rgba8();

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

        let side = self
            .args
            .image_size
            .unwrap_or(if self.args.challenge { 350 } else { 270 });

        save_png(
            src,
            resize_with_scale(self.args.scale, side, side, &self.term),
            &output,
        );

        self.with_backend(
            output
                .to_str()
                .expect("could not convert output path to str"),
        )
    }

    /// creates the wallpaper ascii that fastfetch will display
    pub fn show_wallpaper_ascii(&self, image_arg: &Option<String>) -> PathBuf {
        let img = resize_wallpaper(&self.args, &self.term, image_arg);
        let output_dir = img.parent().expect("could not get output dir");

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
            .status()
            .expect("could not run ascii-image-converter");

        output_dir.join("wfetch-ascii-art.txt")
    }

    pub fn waifu1_default(&self) -> JsonValue {
        self.waifu1(&Rgba8::from(NIX_COLOR1), &Rgba8::from(NIX_COLOR2))
    }

    pub fn waifu2_default(&self) -> JsonValue {
        self.waifu2(&Rgba8::from(NIX_COLOR1), &Rgba8::from(NIX_COLOR2))
    }

    pub fn hollow_default(&self) -> JsonValue {
        json!({
            "source": asset_path("nixos_hollow.txt"),
            "color": json!({ "1": "blue", "2": "cyan" }),
        })
    }

    pub fn smooth_default(&self) -> JsonValue {
        json!({
            "source": asset_path("nixos_smooth.txt"),
            "color": json!({
                "1": "38;5;4", // blue
                "2": "38;5;6", // cyan
                "2": "48;5;6", // blue bg
                "2": "48;5;4", // cyan by
             }),
        })
    }

    pub fn filled_default(&self) -> JsonValue {
        json!({
            "source": "nixos",
            "color": json!({
                "1": "38;5;4", // blue
                "2": "38;5;6", // cyan
                "2": "48;5;6", // blue bg
                "2": "48;5;4", // cyan by
             }),
        })
    }

    pub fn module_for_tmux(&self) -> JsonValue {
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
            return self.hollow_default();
        }

        #[cfg(feature = "nixos")]
        if self.args.smooth {
            return self.smooth_default();
        }

        if self.nixos {
            return self.filled_default();
        }

        // use fastfetch default
        json!({ "source": null })
    }

    pub fn module(&self) -> JsonValue {
        if self.args.wallpaper_ascii.is_some() {
            let ascii_file = self.show_wallpaper_ascii(&self.args.wallpaper_ascii);
            return json!({
                "type": "auto",
                "source": ascii_file.to_str().expect("could not convert ascii file path to str"),
            });
        }

        if self.args.wallpaper.is_some() {
            return self.with_backend(
                resize_wallpaper(&self.args, &self.term, &self.args.wallpaper)
                    .to_str()
                    .expect("could not convert output path to str"),
            );
        }

        // handle tmux separately as the raw xterm sequences breaks rendering and text input
        if self.tmux {
            return self.module_for_tmux();
        }

        match get_term_colors() {
            Err(_) => {
                #[cfg(feature = "nixos")]
                {
                    if self.args.waifu {
                        return self.waifu1_default();
                    }

                    if self.args.waifu2 {
                        return self.waifu2_default();
                    }

                    if self.args.hollow {
                        return self.hollow_default();
                    }

                    if self.args.smooth {
                        return self.smooth_default();
                    }

                    if self.nixos {
                        return self.filled_default();
                    }
                }
            }

            Ok(term_colors) => {
                // remove background color to get contrast
                let (color1, color2) = colors::most_contrasting_pair(&term_colors[1..]);

                #[cfg(feature = "nixos")]
                if self.args.waifu {
                    return self.waifu1(&color1, &color2);
                }
                #[cfg(feature = "nixos")]
                if self.args.waifu2 {
                    return self.waifu2(&color1, &color2);
                }

                let source_colors = json!({
                    "1": color1.term_fg(),
                    "2": color2.term_fg(),
                    "3": color2.term_bg(),
                    "4": color1.term_bg(),
                });

                #[cfg(feature = "nixos")]
                if self.args.hollow {
                    return json!({
                        "source": asset_path("nixos_hollow.txt"),
                        "color": source_colors,
                    });
                }

                #[cfg(feature = "nixos")]
                if self.args.smooth {
                    return json!({
                        "source": asset_path("nixos_smooth.txt"),
                        "color": source_colors,
                    });
                }

                if self.nixos {
                    return json!({
                        "source": "nixos",
                        "color": source_colors,
                    });
                }
            }
        }

        // use fastfetch default
        json!({ "source": null })
    }
}

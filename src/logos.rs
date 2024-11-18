use std::{
    collections::HashMap,
    env,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
};

use fast_image_resize::images::Image;
use fast_image_resize::{IntoImageView, PixelType, ResizeOptions, Resizer};
use image::{codecs::png::PngEncoder, ImageBuffer, ImageEncoder, ImageReader, Rgba};
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};

use crate::colors::get_logo_colors;
use crate::{
    asset_path,
    cli::WFetchArgs,
    colors::{self, Rgba8, Rgba8Ext, TERMINAL_COLORS},
    create_output_file, wallpaper,
};

/// returns new sizes adjusted for the given scale
fn resize_with_scale(scale: Option<f64>, width: u32, height: u32) -> (u32, u32) {
    let scale = scale.unwrap_or_else(|| {
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
            .map_or(1.0, |monitor| monitor.scale)
    });

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    (
        (f64::from(width) * scale).floor() as u32,
        (f64::from(height) * scale).floor() as u32,
    )
}

pub fn image_from_arg(arg: &Option<PathBuf>) -> Option<String> {
    if *arg == Some(PathBuf::from("-")) {
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
pub fn resize_wallpaper(args: &WFetchArgs, image_arg: &Option<PathBuf>) -> PathBuf {
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

    let img = ImageReader::open(&wall)
        .expect("could not open image")
        .decode()
        .expect("could not decode image");

    let dst_size = args
        .image_size
        .unwrap_or(if args.challenge { 350 } else { 270 });

    let (dst_size, _) = resize_with_scale(args.scale, dst_size, dst_size);

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
        std::fs::File::create(output).unwrap_or_else(|_| panic!("could not create {output:?}")),
    );

    #[allow(clippy::cast_sign_loss)]
    PngEncoder::new(&mut result_buf)
        .write_image(dest.buffer(), dst_w, dst_h, image::ColorType::Rgba8.into())
        .unwrap_or_else(|_| panic!("failed to write png for {output:?}"));
}

pub struct Logo {
    args: WFetchArgs,
    nixos: bool,
}

impl Logo {
    pub fn new(args: &WFetchArgs, nixos: bool) -> Self {
        Self {
            args: args.clone(),
            nixos,
        }
    }

    fn with_backend(source: &str) -> JsonValue {
        let logo_backend = match env::var("KONSOLE_VERSION") {
            Ok(_) => "iterm",
            Err(_) => "kitty-direct",
        };

        json!({
            "type": logo_backend,
            "source": source,
            "preserveAspectRatio": true,
        })
    }

    pub fn waifu1(&self, color1: &Rgba8, color2: &Rgba8) -> JsonValue {
        let output = create_output_file("wfetch.png");

        let replace1 = Rgba([0x52, 0x78, 0xc3, 255]);
        let replace2 = Rgba([0x7f, 0xba, 0xe4, 255]);

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

        save_png(src, resize_with_scale(self.args.scale, side, side), &output);

        Self::with_backend(
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

        save_png(src, resize_with_scale(self.args.scale, side, side), &output);

        Self::with_backend(
            output
                .to_str()
                .expect("could not convert output path to str"),
        )
    }

    /// creates the wallpaper ascii that fastfetch will display
    pub fn show_wallpaper_ascii(&self, image_arg: &Option<PathBuf>) -> PathBuf {
        let img = resize_wallpaper(&self.args, image_arg);
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

    pub fn module(&self) -> JsonValue {
        if self.args.wallpaper_ascii.is_some() {
            let ascii_file = self.show_wallpaper_ascii(&self.args.wallpaper_ascii);
            return json!({
                "type": "auto",
                "source": ascii_file.to_str().expect("could not convert ascii file path to str"),
            });
        }

        if self.args.wallpaper.is_some() {
            return Self::with_backend(
                resize_wallpaper(&self.args, &self.args.wallpaper)
                    .to_str()
                    .expect("could not convert output path to str"),
            );
        }

        match get_logo_colors() {
            Err(_) => {
                #[cfg(feature = "nixos")]
                {
                    assert!(
                        !(self.args.waifu || self.args.waifu2),
                        "failed to get logo colors"
                    );

                    if self.args.hollow {
                        let hollow = asset_path("nixos_hollow.txt");
                        return json!({
                            "source": hollow,
                            "color": json!({ "1": "blue", "2": "cyan" }),
                        });
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

                // get named colors to pass to fastfetch
                let named_colors: HashMap<_, _> = term_colors.iter().zip(TERMINAL_COLORS).collect();
                let source_colors = json!({
                    "1": (*named_colors.get(&color1).unwrap_or(&"blue")).to_string(),
                    "2": (*named_colors.get(&color2).unwrap_or(&"cyan")).to_string(),
                });

                #[cfg(feature = "nixos")]
                if self.args.hollow {
                    let hollow = asset_path("nixos_hollow.txt");
                    return json!({
                        "source": hollow,
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

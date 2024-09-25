use std::path::PathBuf;
use std::process::Stdio;
use std::{collections::HashMap, env};

use execute::command_args;
use fast_image_resize::images::Image;
use fast_image_resize::{IntoImageView, PixelType, ResizeOptions, Resizer};
use image::codecs::png::PngEncoder;
use image::{ImageBuffer, ImageEncoder, ImageReader, Rgba};
use serde_json::{json, Value as JsonValue};

use crate::colors::get_logo_colors;
use crate::{
    asset_path,
    cli::WFetchArgs,
    colors::{self, Rgba8, Rgba8Ext, TERMINAL_COLORS},
    create_output_file,
    wallpaper::{self, WallInfo},
};

/// creates the wallpaper image that fastfetch will display
pub fn resize_wallpaper(args: &WFetchArgs) -> PathBuf {
    let output = create_output_file("wfetch.png");

    // read current wallpaper
    let wall = wallpaper::detect(&args.wallpaper).unwrap_or_else(|| {
        eprintln!("Error: could not detect wallpaper!");
        std::process::exit(1);
    });

    let wall_info = wallpaper::info(&wall);

    let img = ImageReader::open(wall)
        .expect("could not open image")
        .decode()
        .expect("could not decode image");

    let fallback_crop = {
        let width = f64::from(img.width());
        let height = f64::from(img.height());

        // get basic square crop in the center
        if width > height {
            (height, height, (width - height) / 2.0, 0.0)
        } else {
            (width, width, 0.0, (height - width) / 2.0)
        }
    };

    let (w, h, x, y) = if let Some(WallInfo {
        r1x1: crop_area, ..
    }) = &wall_info
    {
        let geometry: Vec<_> = crop_area
            .split(|c| c == '+' || c == 'x')
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();

        match geometry.as_slice() {
            &[w, h, x, y] => (w, h, x, y),
            _ => fallback_crop,
        }
    } else {
        fallback_crop
    };

    let dst_size = args
        .image_size
        .unwrap_or(if args.challenge { 350 } else { 270 });

    #[allow(clippy::cast_sign_loss)]
    let mut dest = Image::new(
        dst_size,
        dst_size,
        img.pixel_type().expect("could not get pixel type"),
    );
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

        save_png(src, (side, side), &output);

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

        save_png(src, (side, side), &output);

        Self::with_backend(
            output
                .to_str()
                .expect("could not convert output path to str"),
        )
    }

    /// creates the wallpaper ascii that fastfetch will display
    pub fn show_wallpaper_ascii(&self) -> PathBuf {
        let img = resize_wallpaper(&self.args);
        let output_dir = img.parent().expect("could not get output dir");

        // NOTE: uses patched version of ascii-image-converter to be able to output colored ascii text to file
        command_args!(
            "ascii-image-converter",
            "--color",
            "--braille",
            "--threshold",
            "50"
        )
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
        .expect("could not execute ascii-image-converter");

        output_dir.join("wfetch-ascii-art.txt")
    }

    pub fn module(&self) -> JsonValue {
        if self.args.wallpaper_ascii.is_some() {
            let ascii_file = self.show_wallpaper_ascii();

            return json!({
                "type": "auto",
                "source": ascii_file.to_str().expect("could not convert ascii file path to str"),
            });
        }

        if self.args.wallpaper.is_some() {
            return Self::with_backend(
                resize_wallpaper(&self.args)
                    .to_str()
                    .expect("could not convert output path to str"),
            );
        }

        match get_logo_colors() {
            Err(_) => {
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

            Ok(term_colors) => {
                // remove background color to get contrast
                let (color1, color2) = colors::most_contrasting_pair(&term_colors[1..]);

                if self.args.waifu {
                    return self.waifu1(&color1, &color2);
                }
                if self.args.waifu2 {
                    return self.waifu2(&color1, &color2);
                }

                // get named colors to pass to fastfetch
                let named_colors: HashMap<_, _> = term_colors.iter().zip(TERMINAL_COLORS).collect();
                let source_colors = json!({
                    "1": (*named_colors.get(&color1).unwrap_or(&"blue")).to_string(),
                    "2": (*named_colors.get(&color2).unwrap_or(&"cyan")).to_string(),
                });

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

use std::{collections::HashMap, env, str::FromStr};

use execute::Execute;
use fast_image_resize::images::Image;
use fast_image_resize::{IntoImageView, PixelType, ResizeOptions, Resizer};
use image::codecs::png::PngEncoder;
use image::{ImageEncoder, ImageReader};
use serde_json::{json, Value as JsonValue};

use crate::{
    asset_path,
    cli::WFetchArgs,
    colors::{self, Color, TERMINAL_COLORS},
    create_output_file, full_path,
    wallpaper::{self, WallInfo},
    WFetchResult,
};

#[derive(serde::Deserialize)]
struct NixInfo {
    colors: HashMap<String, String>,
}

fn logo_colors_from_json() -> WFetchResult<Vec<Color>> {
    let contents = std::fs::read_to_string(full_path("~/.cache/wallust/nix.json"))?;

    let colors = serde_json::from_str::<NixInfo>(&contents)?.colors;

    (0..16)
        .map(|i| {
            let color_str = colors
                .get(&format!("color{i}"))
                .ok_or("failed to get color")?;
            let color = Color::from_str(color_str)?;
            Ok(color)
        })
        .collect()
}

fn logo_colors_from_xterm() -> WFetchResult<Vec<Color>> {
    (0..16).map(crate::xterm::query_term_color).collect()
}

pub fn get_logo_colors() -> WFetchResult<Vec<Color>> {
    logo_colors_from_json().or_else(|_| logo_colors_from_xterm())
}

/// gets the image
fn image_resize_args(args: &WFetchArgs, smaller_size: i32) -> Vec<String> {
    let size = args.image_size.unwrap_or(if args.challenge {
        smaller_size + 80
    } else {
        smaller_size
    });

    vec!["-resize".to_string(), format!("{size}x{size}")]
}

/// creates the wallpaper image that fastfetch will display
pub fn resize_wallpaper(args: &WFetchArgs) -> String {
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
        // let (width, height) =
        //     image::image_dimensions(&wall).expect("could not get image dimensions");
        let width = f64::from(img.width());
        let height = f64::from(img.height());

        // get square crop for imagemagick
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
        .unwrap_or(if args.challenge { 380 } else { 300 });

    #[allow(clippy::cast_sign_loss)]
    let mut dest = Image::new(
        dst_size as u32,
        dst_size as u32,
        img.pixel_type().expect("could not get pixel type"),
    );
    Resizer::new()
        .resize(&img, &mut dest, &ResizeOptions::new().crop(x, y, w, h))
        .expect("failed to resize image");

    let mut result_buf =
        std::io::BufWriter::new(std::fs::File::create(&output).expect("could not create file"));

    #[allow(clippy::cast_sign_loss)]
    PngEncoder::new(&mut result_buf)
        .write_image(
            dest.buffer(),
            dst_size as u32,
            dst_size as u32,
            img.color().into(),
        )
        .expect("failed to write wallpaper crop");

    output
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

    pub fn waifu1(&self, color1: &Color, color2: &Color) -> JsonValue {
        let output = create_output_file("wfetch.png");

        let replace1: Color = "#5278c3".parse().expect("unable to parse hex color");
        let replace2: Color = "#7fbae4".parse().expect("unable to parse hex color");

        let mut img = ImageReader::open(asset_path("nixos1.png"))
            .expect("could not open image")
            .decode()
            .expect("could not decode image")
            .into_rgba8();

        let fuzz = 0.1 * (255.0_f64 * 255.0_f64 * 3.0_f64).sqrt();

        for pixel in img.pixels_mut() {
            if replace1.distance_rgba(pixel) < fuzz {
                *pixel = color1.to_rgba(pixel.0[3]);
            } else if replace2.distance_rgba(pixel) < fuzz {
                *pixel = color2.to_rgba(pixel.0[3]);
            }
        }

        let dest_h = self
            .args
            .image_size
            .unwrap_or(if self.args.challenge { 380 } else { 300 });

        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        let dest_w = (f64::from(dest_h) / f64::from(img.height()) * f64::from(img.width())) as u32;

        let src_view =
            Image::from_vec_u8(img.width(), img.height(), img.into_raw(), PixelType::U8x4)
                .expect("could not create image view");

        #[allow(clippy::cast_sign_loss)]
        let mut dest = Image::new(dest_w, dest_h as u32, PixelType::U8x4);
        Resizer::new()
            .resize(&src_view, &mut dest, None)
            .expect("failed to resize image");

        let mut result_buf =
            std::io::BufWriter::new(std::fs::File::create(&output).expect("could not create file"));

        #[allow(clippy::cast_sign_loss)]
        PngEncoder::new(&mut result_buf)
            .write_image(
                dest.buffer(),
                dest_w,
                dest_h as u32,
                image::ColorType::Rgba8.into(),
            )
            .expect("failed to write png for waifu1");

        Self::with_backend(&output)
    }

    pub fn waifu2(&self, color1: &Color, color2: &Color) -> JsonValue {
        let output = create_output_file("wfetch.png");

        execute::command_args!("convert", &asset_path("nixos2.png"),)
            // color 1 using mask1
            .args([
                &asset_path("nixos2-mask1.jpg"),
                "-compose",
                "Multiply",
                "-composite",
            ])
            .args(color1.imagemagick_replace_args("black"))
            // color 2 using mask2
            .args([
                &asset_path("nixos2-mask2.jpg"),
                "-compose",
                "Multiply",
                "-composite",
            ])
            .args(color2.imagemagick_replace_args("black"))
            // set transparency using original image
            .args([
                &asset_path("nixos2.png"),
                "-compose",
                "CopyOpacity",
                "-composite",
            ])
            // finally resize
            .args(image_resize_args(&self.args, 305))
            .arg(&output)
            .execute()
            .expect("failed to create nixos logo");

        Self::with_backend(&output)
    }

    pub fn module(&self) -> JsonValue {
        // from wallpaper, no need to compute contrasting colors
        if self.args.wallpaper_ascii.is_some() {
            return json!({ "type": "auto", "source": "-" });
        }
        if self.args.wallpaper.is_some() {
            return Self::with_backend(&resize_wallpaper(&self.args));
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

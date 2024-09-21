use std::{collections::HashMap, env, process::Command, str::FromStr};

use execute::Execute;
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

pub fn imagemagick_wallpaper(args: &WFetchArgs, wallpaper_arg: &Option<String>) -> Command {
    // read current wallpaper
    let wall = wallpaper::detect(wallpaper_arg).unwrap_or_else(|| {
        eprintln!("Error: could not detect wallpaper!");
        std::process::exit(1);
    });

    let wallpaper_info = wallpaper::info(&wall);

    let crop_area = if let Some(WallInfo {
        r1x1: crop_area, ..
    }) = &wallpaper_info
    {
        crop_area.to_owned()
    } else {
        let (width, height) =
            image::image_dimensions(&wall).expect("could not get image dimensions");

        // get square crop for imagemagick
        if width > height {
            format!("{height}x{height}+{}+0", (width - height) / 2)
        } else {
            format!("{width}x{width}+0+{}", (height - width) / 2)
        }
    };

    let image_size = args
        .image_size
        .unwrap_or(if args.challenge { 380 } else { 300 });

    // use imagemagick to crop and resize the wallpaper
    execute::command_args!(
        "magick",
        wall,
        "-strip",
        "-crop",
        crop_area,
        "-resize",
        format!("{image_size}x{image_size}"),
    )
}

/// creates the wallpaper image that fastfetch will display
fn create_wallpaper_image(args: &WFetchArgs) -> String {
    let output = create_output_file("wfetch.png");

    imagemagick_wallpaper(args, &args.wallpaper)
        .arg(&output)
        .execute()
        .expect("failed to execute imagemagick");

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

        execute::command_args!(
            "magick",
            // replace color 1
            &asset_path("nixos1.png"),
        )
        .args(color1.imagemagick_replace_args("#5278c3"))
        .args(color2.imagemagick_replace_args("#7fbae4"))
        .args(image_resize_args(&self.args, 340))
        // .arg("-strip")
        .arg("-compress")
        .arg("None")
        .arg(&output)
        .execute()
        .expect("failed to create nixos logo");

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
            return Self::with_backend(&create_wallpaper_image(&self.args));
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

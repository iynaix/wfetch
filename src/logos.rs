use std::{collections::HashMap, str::FromStr};

use execute::Execute;

use crate::{
    asset_path, cli::WFetchArgs, colors::Color, create_output_file, full_path, WFetchResult,
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

pub fn create_nixos_logo1(args: &WFetchArgs, color1: &Color, color2: &Color) -> String {
    let output = create_output_file("wfetch.png");

    execute::command_args!(
        "magick",
        // replace color 1
        &asset_path("nixos1.png"),
    )
    .args(color1.imagemagick_replace_args("#5278c3"))
    .args(color2.imagemagick_replace_args("#7fbae4"))
    .args(image_resize_args(args, 340))
    // .arg("-strip")
    .arg("-compress")
    .arg("None")
    .arg(&output)
    .execute()
    .expect("failed to create nixos logo");

    output
}

pub fn create_nixos_logo2(args: &WFetchArgs, color1: &Color, color2: &Color) -> String {
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
        .args(image_resize_args(args, 305))
        .arg(&output)
        .execute()
        .expect("failed to create nixos logo");

    output
}

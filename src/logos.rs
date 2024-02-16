use execute::Execute;
use std::collections::HashMap;

use crate::{asset_path, cli::WFetchArgs, create_output_file, full_path};

#[derive(serde::Deserialize)]
struct NixInfo {
    colors: HashMap<String, String>,
}

fn get_logo_colors() -> (String, String) {
    let contents = std::fs::read_to_string(full_path("~/.cache/wallust/nix.json"))
        .unwrap_or_else(|_| panic!("failed to load nix.json"));

    let hexless = serde_json::from_str::<NixInfo>(&contents)
        .unwrap_or_else(|_| panic!("failed to parse nix.json"))
        .colors;

    let c1 = hexless.get("color4").expect("invalid color");
    let c2 = hexless.get("color6").expect("invalid color");

    (c1.to_owned(), c2.to_owned())
}

/// gets the image
fn get_image_size(args: &WFetchArgs, smaller_size: i32) -> i32 {
    args.image_size.unwrap_or(if args.challenge {
        smaller_size + 80
    } else {
        smaller_size
    })
}

fn fill_color_args(fill: &str, opaque: &str) -> Vec<String> {
    ["-fuzz", "10%", "-fill", &fill, "-opaque", &opaque]
        .iter()
        .map(std::string::ToString::to_string)
        .collect()
}

pub fn create_nixos_logo1(args: &WFetchArgs) -> String {
    let (c1, c2) = get_logo_colors();

    let output = create_output_file(format!("nixos1-{c1}-{c2}.png"));
    let image_size = get_image_size(args, 340);

    execute::command_args!(
        "convert",
        // replace color 1
        &asset_path("nixos1.png"),
    )
    .args(fill_color_args(&c1, "#5278c3"))
    .args(fill_color_args(&c2, "#7fbae4"))
    .args(["-resize", &format!("{image_size}x{image_size}"), &output])
    .execute()
    .expect("failed to create nixos logo");

    output
}

pub fn create_nixos_logo2(args: &WFetchArgs) -> String {
    let (c1, c2) = get_logo_colors();

    let output = create_output_file(format!("nixos2-{c1}-{c2}.png"));
    let image_size = get_image_size(args, 305);

    execute::command_args!("convert", &asset_path("nixos2.png"),)
        // color 1 using mask1
        .args([
            &asset_path("nixos2-mask1.jpg"),
            "-compose",
            "Multiply",
            "-composite",
        ])
        .args(fill_color_args(&c1, "black"))
        // color 2 using mask2
        .args([
            &asset_path("nixos2-mask2.jpg"),
            "-compose",
            "Multiply",
            "-composite",
        ])
        .args(fill_color_args(&c2, "black"))
        // set transparency using original image
        .args([
            &asset_path("nixos2.png"),
            "-compose",
            "CopyOpacity",
            "-composite",
        ])
        // finally resize
        .args(["-resize", &format!("{image_size}x{image_size}"), &output])
        .execute()
        .expect("failed to create nixos logo");

    output
}

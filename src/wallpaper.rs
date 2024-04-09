use execute::Execute;
use serde::Deserialize;
use std::{path::Path, process::Stdio};

use crate::{full_path, CommandUtf8};

#[derive(Debug, Default, Deserialize, Clone)]
pub struct Face {
    #[serde(rename = "0")]
    pub xmin: u32,
    #[serde(rename = "1")]
    pub xmax: u32,
    #[serde(rename = "2")]
    pub ymin: u32,
    #[serde(rename = "3")]
    pub ymax: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WallInfo {
    pub filename: String,
    pub width: u32,
    pub height: u32,
    #[serde(rename = "1x1")]
    pub r1x1: String,
}

/// reads the wallpaper info from wallpapers.csv
pub fn info(image: &String) -> Option<WallInfo> {
    let wallpapers_csv = full_path("~/Pictures/Wallpapers/wallpapers.csv");
    if !wallpapers_csv.exists() {
        return None;
    }

    // convert image to path
    let image = Path::new(image);
    let fname = image
        .file_name()
        .expect("invalid image path")
        .to_str()
        .expect("could not convert image path to str");

    let reader = std::io::BufReader::new(
        std::fs::File::open(wallpapers_csv).expect("could not open wallpapers.csv"),
    );

    let mut rdr = csv::Reader::from_reader(reader);
    rdr.deserialize::<WallInfo>()
        .flatten()
        .find(|line| line.filename == fname)
        .map(|mut info| {
            // calculate square to crop from image dimensions
            let size = std::cmp::min(info.width, info.height);
            info.r1x1 = format!("{size}x{size}+{}", info.r1x1,);
            info
        })
}

/// detect wallpaper using swwww
fn detect_iynaixos() -> Option<String> {
    std::fs::read_to_string(
        dirs::runtime_dir()
            .expect("could not get XDG_RUNTIME_DIR")
            .join("current_wallpaper"),
    )
    .ok()
    .filter(|wallpaper| !wallpaper.is_empty())
}

/// detect wallpaper using swwww
fn detect_swww() -> Option<String> {
    execute::command!("swww query")
        .execute_stdout_lines()
        .first()
        .map(|wallpaper| {
            wallpaper
                .rsplit_once("image: ")
                .unwrap_or_default()
                .1
                .trim()
                .trim_matches('\'')
                .to_string()
        })
        .filter(|wallpaper| !wallpaper.is_empty() && wallpaper != "STDIN")
}

// detect wallpaper using swaybg
fn detect_swaybg() -> Option<String> {
    let sys = sysinfo::System::new_all();

    if let Some(process) = sys.processes_by_exact_name("swaybg").next() {
        if let Some(wallpaper) = process.cmd().last() {
            return Some(wallpaper.to_string());
        }
    }

    None
}

fn detect_hyprpaper() -> Option<String> {
    std::fs::read_to_string(full_path("~/.config/hypr/hyprpaper.conf"))
        .ok()
        .and_then(|conf| {
            conf.lines()
                .map(str::trim)
                .find(|line| line.starts_with("wallpaper"))
                .and_then(|line| {
                    line.rsplit_once(',')
                        .map(|(_, wallpaper)| wallpaper.trim().to_string())
                })
        })
}

/// detect wallpaper using gsettings (gnome, cinnamon, mate)
fn detect_gsettings() -> Option<String> {
    [
        ("org.gnome.desktop.background", "picture-uri"),
        ("org.cinnamon.desktop.background", "picture-uri"),
        ("org.mate.background", "picture-filename"),
    ]
    .iter()
    .find_map(|(gdir, gkey)| {
        execute::command_args!("gsettings", "get", gdir, gkey)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .execute_output()
            .ok()
            .map(|output| String::from_utf8(output.stdout).unwrap_or_default())
            .map(|wallpaper| {
                let wallpaper = wallpaper.trim();
                wallpaper
                    .trim_matches('\'')
                    .strip_prefix("file://")
                    .unwrap_or(wallpaper)
                    .to_string()
            })
    })
}

/// returns full path to the wallpaper
pub fn detect(wallpaper_arg: &Option<String>) -> Option<String> {
    [
        // wallpaper provided in arguments
        wallpaper_arg.clone().filter(|w| !w.is_empty()),
        detect_iynaixos(),
        detect_swww(),
        detect_swaybg(),
        detect_hyprpaper(),
        detect_gsettings(), // gnome / cinnamon / mate
    ]
    .iter()
    .find(|&wallpaper| wallpaper.is_some())
    .and_then(std::clone::Clone::clone)
}

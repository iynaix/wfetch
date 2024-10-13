use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

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
    Command::new("swww query")
        .execute_stdout_lines()
        .first()
        .and_then(|wallpaper| wallpaper.rsplit_once("image: "))
        .map(|(_, wallpaper)| wallpaper.trim().trim_matches('\'').to_string())
        .filter(|wallpaper| !wallpaper.is_empty() && wallpaper != "STDIN")
}

// detect wallpaper using swaybg
fn detect_swaybg() -> Option<String> {
    let sys = sysinfo::System::new_all();

    let mut processes = sys.processes_by_exact_name("swaybg".as_ref());
    processes
        .find_map(|process| process.cmd().last().cloned())
        .and_then(|wallpaper| wallpaper.into_string().ok())
}

fn detect_hyprpaper() -> Option<String> {
    std::fs::read_to_string(full_path("~/.config/hypr/hyprpaper.conf"))
        .ok()?
        .lines()
        .find(|line| line.trim().starts_with("wallpaper"))?
        .rsplit_once(',')?
        .1
        .trim()
        .to_string()
        .into()
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
        Command::new("gsettings")
            .arg("get")
            .arg(gdir)
            .arg(gkey)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
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

fn detect_plasma() -> Option<String> {
    let plasma_script = r#"print(desktops().map(d => {d.currentConfigGroup=["Wallpaper", "org.kde.image", "General"]; return d.readConfig("Image")}).join("\n"))"#;
    Command::new("qdbus")
        .arg("org.kde.plasmashell")
        .arg("/PlasmaShell")
        .arg("org.kde.PlasmaShell.evaluateScript")
        .arg(plasma_script)
        .execute_stdout_lines()
        .first()
        .map(|wallpaper| {
            wallpaper
                .strip_prefix("file://")
                .unwrap_or(wallpaper)
                .to_string()
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
        detect_plasma(),    // kde
    ]
    .iter()
    .find(|&wallpaper| {
        if let Some(wall) = wallpaper {
            return PathBuf::from(wall).exists();
        }
        false
    })
    .and_then(std::clone::Clone::clone)
}

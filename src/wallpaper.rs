use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{CommandUtf8, full_path};

/// detect wallpaper using current-wallpaper file in tmpfs
pub fn detect_iynaixos() -> Option<String> {
    std::fs::read_to_string(
        dirs::runtime_dir()
            .expect("could not get XDG_RUNTIME_DIR")
            .join("current_wallpaper"),
    )
    .ok()
    .filter(|wallpaper| !wallpaper.is_empty())
}

pub fn geom_from_str(crop: &str) -> Option<(f64, f64, f64, f64)> {
    let geometry: Vec<_> = crop
        .split(['+', 'x'])
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();

    match geometry.as_slice() {
        &[w, h, x, y] => Some((w, h, x, y)),
        _ => None,
    }
}

/// reads the wallpaper info from image xmp metadata (w, h, x, y)
pub fn info(image: &str, fallback: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
    use rexiv2::Metadata;

    let meta = Metadata::new_from_path(image).expect("could not init new metadata");

    meta.get_tag_string("Xmp.wallfacer.crop.1x1").map_or_else(
        |_| fallback,
        |crop| geom_from_str(&crop).unwrap_or(fallback),
    )
}

/// detect wallpaper using swwww
fn detect_swww() -> Option<String> {
    Command::new("swww")
        .arg("query")
        .execute_stdout_lines()
        .first()
        .and_then(|wallpaper| wallpaper.rsplit_once("image: "))
        .map(|(_, wallpaper)| wallpaper.trim().trim_matches('\'').to_string())
        .filter(|wallpaper| !wallpaper.is_empty() && wallpaper != "STDIN")
}

/// detect wallpaper using swaybg
fn detect_swaybg() -> Option<String> {
    let sys = sysinfo::System::new_all();

    let mut processes = sys.processes_by_exact_name("swaybg".as_ref());
    processes
        .find_map(|process| process.cmd().last().cloned())
        .and_then(|wallpaper| wallpaper.into_string().ok())
}

/// detect wallpaper using hyprpaper
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

/// detect wallpaper for plasma
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

/// detect wallpaper for noctalia shell
fn detect_noctalia() -> Option<String> {
    #[derive(Debug, serde::Deserialize)]
    pub struct NoctaliaState {
        pub state: NoctaliaWallpaperState,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct NoctaliaWallpaperState {
        pub wallpapers: std::collections::HashMap<String, String>,
    }

    fn try_noctalia_command(cmd: &mut Command) -> Option<String> {
        let output = cmd.output().ok()?;

        if !output.status.success() {
            return None;
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let noctalia_state: NoctaliaState = serde_json::from_str(&output_str).ok()?;

        noctalia_state.state.wallpapers.values().next().cloned()
    }

    try_noctalia_command(Command::new("noctalia-shell").args(["ipc", "call", "state", "all"]))
        .or_else(|| {
            try_noctalia_command(
                Command::new("qs")
                    .args(["-c", "noctalia-shell"])
                    .args(["ipc", "call", "state", "all"]),
            )
        })
}

/// detect wallpaper for dank material shell
fn detect_dms() -> Option<String> {
    let output = Command::new("dms")
        .args(["ipc", "call", "wallpaper", "get"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

/// returns full path to the wallpaper
pub fn detect<P>(wallpaper_arg: &Option<P>) -> Option<String>
where
    P: AsRef<Path>,
{
    [
        // wallpaper provided in arguments
        wallpaper_arg
            .as_ref()
            .and_then(|s| s.as_ref().to_str().map(std::string::ToString::to_string)),
        detect_iynaixos(),
        detect_swww(),
        detect_swaybg(),
        detect_hyprpaper(),
        detect_noctalia(),
        detect_dms(),
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

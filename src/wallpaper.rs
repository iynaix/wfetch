use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{CommandUtf8, full_path};

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
    let wallfacer_ns = "http://example.com/wallfacer/";

    let mut fp = xmpkit::XmpFile::new();
    fp.open(&image).expect("failed to open image");

    fp.get_xmp().map_or(fallback, |xmp| {
        xmp.get_struct_field(wallfacer_ns, "crops", "1x1")
            .and_then(|crop| crop.as_str().and_then(geom_from_str))
            .unwrap_or(fallback)
    })
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
pub fn detect_noctalia() -> Option<String> {
    let cmd = Command::new("noctalia")
        .arg("msg")
        .arg("wallpaper-get")
        .stdout(Stdio::piped())
        .output()
        .ok()?;

    String::from_utf8(cmd.stdout)
        .ok()
        .map(|s| s.trim().to_string())
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
        detect_noctalia(),
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

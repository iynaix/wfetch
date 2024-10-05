use crate::cli::WFetchArgs;
use chrono::{DateTime, Datelike, NaiveDate, Timelike};
use logos::Logo;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

pub mod cli;
pub mod colors;
pub mod logos;
pub mod wallpaper;
pub mod xterm;

pub type WFetchResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn full_path<P>(p: P) -> PathBuf
where
    P: AsRef<std::path::Path>,
{
    let p = p.as_ref().to_str().expect("invalid path");

    match p.strip_prefix("~/") {
        Some(p) => dirs::home_dir().expect("invalid home directory").join(p),
        None => PathBuf::from(p),
    }
}

pub trait CommandUtf8 {
    fn execute_stdout_lines(&mut self) -> Vec<String>;
}

impl CommandUtf8 for std::process::Command {
    fn execute_stdout_lines(&mut self) -> Vec<String> {
        self.stdout(Stdio::piped()).output().map_or_else(
            |_| Vec::new(),
            |output| {
                String::from_utf8(output.stdout)
                    .expect("invalid utf8 from command")
                    .lines()
                    .map(String::from)
                    .collect()
            },
        )
    }
}

pub fn asset_path(filename: &str) -> String {
    let out_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
        env::current_exe()
            .expect("could not get current dir")
            .ancestors()
            .nth(2)
            .expect("could not get base package dir")
            .to_str()
            .expect("could not convert base package dir to str")
            .to_string()
    }));
    let asset = out_path.join("assets").join(filename);
    asset
        .to_str()
        .unwrap_or_else(|| panic!("could not get asset {}", &filename))
        .to_string()
}

pub fn create_output_file(filename: &str) -> PathBuf {
    let output_dir = full_path("/tmp/wfetch");
    std::fs::create_dir_all(&output_dir).expect("failed to create output dir");

    output_dir.join(filename)
}

fn term_color(color: i32, text: &String, bold: bool) -> String {
    let bold_format = if bold { "1;" } else { "" };
    format!("\u{1b}[{bold_format}{}m{text}\u{1b}[0m", 30 + color)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let month = if month == 12 { 1 } else { month };
    let year = if month == 12 { year + 1 } else { year };

    let first_day_of_next_month =
        NaiveDate::from_ymd_opt(year, month + 1, 1).expect("cannot create ymd");
    (first_day_of_next_month - chrono::Duration::try_days(1).expect("cannot create duration")).day()
}

#[derive(Debug)]
pub struct Fastfetch {
    args: WFetchArgs,
    preprocess: HashMap<String, String>,
}

impl Fastfetch {
    pub fn new(args: &WFetchArgs) -> Self {
        let preprocess: HashMap<_, _> = Command::new("fastfetch")
            .arg("--config")
            .arg(asset_path("preprocess.json"))
            .execute_stdout_lines()
            .iter()
            .map(|l| {
                let (k, v) = l.split_once(": ").unwrap_or_default();
                (k.to_string(), v.to_string())
            })
            .collect();

        Self {
            preprocess,
            args: args.clone(),
        }
    }

    // gets a value from preprocessed, given its key
    fn preprocess(&self, key: &str) -> String {
        self.preprocess.get(key).cloned().unwrap_or(String::new())
    }

    fn os_module(&self) -> serde_json::Value {
        let os = self.preprocess("OS").to_lowercase();

        for (s, key) in &[
            ("nixos", ""),
            ("kali", ""),
            ("rocky", ""),
            ("mint", "󰣭"),
            ("alpine", ""),
            ("archcraft", ""),
            ("archlabs", ""),
            ("arcolinux", ""),
            ("artix", ""),
            ("centos", ""),
            ("coreos", ""),
            ("crystal", ""),
            ("debian", ""),
            ("deepin", ""),
            ("devuan", ""),
            ("elementary", ""),
            ("endeavour", ""),
            ("fedora", ""),
            ("macos", ""),
            ("freebsd", ""),
            ("garuda", ""),
            ("gentoo", ""),
            ("hyperbola", ""),
            ("illumos", ""),
            ("kubuntu", ""),
            ("locos", ""),
            ("mageia", ""),
            ("mandriva", ""),
            ("manjaro", ""),
            ("mxlinux", ""),
            ("openbsd", ""),
            ("opensuse", ""),
            ("parabola", ""),
            ("parrot", ""),
            ("puppy", ""),
            ("qubes", ""),
            ("redhat", ""),
            ("sabayon", ""),
            ("slackware", ""),
            ("solus", ""),
            ("tails", ""),
            ("trisquel", ""),
            ("ubuntu", ""),
            ("vanilla", ""),
            ("void", ""),
            ("xerolinux", ""),
            ("xorin", ""),
            ("guix", ""),
            ("pop!_os", ""),
            ("rhel", ""),
            ("arch", ""),
            ("alma", ""),
        ] {
            if os.contains(s) {
                return json!({
                    "type": "os",
                    "key": format!("{key} OS"),
                    "format": "{3}"
                });
            }
        }

        json!({
            "type": "os",
            "key": format!(" OS"),
            "format": "{3}"
        })
    }

    fn wm_module(&self) -> serde_json::Value {
        let de = self.preprocess("DE").to_lowercase();
        let wm = self.preprocess("WM").to_lowercase();

        for (s, key) in &[
            ("hyprland", ""),
            ("awesome", ""),
            ("bspwm", ""),
            ("budgie", ""),
            ("cinnamon", ""),
            ("dwm", ""),
            ("enlightenment", ""),
            ("fluxbox", ""),
            ("gnome", ""),
            ("i3", ""),
            ("lxde", ""),
            ("lxqt", ""),
            ("mate", ""),
            ("plasma", ""),
            ("qtile", ""),
            ("sway", ""),
            ("xfce", ""),
            ("xmonad", ""),
        ] {
            if de.contains(s) {
                return json!({ "type": "de", "key": format!("{key} DE"), "format": "{2} ({3})" });
            }
            if wm.contains(s) {
                return json!({ "type": "wm", "key": format!("{key} WM"), "format": "{2}" });
            }
        }

        json!({ "type": "wm", "key": format!("󰕮 WM"), "format": "{2}" })
    }

    #[allow(clippy::unused_self)]
    fn shell_module(&self) -> serde_json::Value {
        if let Ok(starship_shell) = std::env::var("STARSHIP_SHELL") {
            if starship_shell.ends_with("fish") {
                return json!({
                    "type": "command",
                    "key": "󰈺 SH",
                    "text": "echo fish",
                });
            }
            return json!({
                "type": "command",
                "key": " SH",
                "text": format!("echo {starship_shell}"),
            });
        }

        let shell = std::env::var("SHELL").unwrap_or_default();

        if shell.ends_with("fish") {
            return json!({
                "type": "command",
                "key": "󰈺 SH",
                "text": "echo fish",
            });
        }

        if shell.ends_with("zsh") {
            return json!({
                "type": "command",
                "key": " SH",
                "text": "echo zsh",
            });
        }

        json!({
            "type": "command",
            "key": " SH",
            "text": "echo bash",
        })
    }

    fn logo_module(&self) -> serde_json::Value {
        Logo::new(&self.args, self.preprocess("OS").contains("NixOS")).module()
    }

    fn gpu_module(&self) -> Vec<serde_json::Value> {
        let (discrete, other): (Vec<_>, Vec<_>) = self
            .preprocess
            .iter()
            .filter_map(|(k, v)| k.starts_with("GPU").then_some(v))
            .map(|v| {
                let (gpu, gpu_type) = v.split_once("____").expect("invalid gpu format");
                (
                    json!({
                        "type": "command",
                        "key": " GPU",
                        "text": format!("echo {gpu}"),
                    }),
                    gpu_type == "Discrete",
                )
            })
            .partition(|(_, is_discrete)| *is_discrete);

        let ret = if discrete.is_empty() {
            // return everything except first gpu
            if other.len() > 1 {
                other[1..].to_vec()
            } else {
                other
            }
        } else {
            discrete
        };

        ret.into_iter().map(|(gpu, _)| gpu).collect()
    }

    #[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
    fn challenge_text(&self) -> String {
        let start = DateTime::parse_from_str(&self.args.challenge_timestamp.to_string(), "%s")
            .expect("could not parse start timestamp");

        let mths = self.args.challenge_months % 12;
        let yrs = self.args.challenge_years + self.args.challenge_months / 12;

        let final_mth = if start.month() + mths > 12 {
            start.month() + mths - 12
        } else {
            start.month() + mths
        };
        let final_yr = if start.month() + mths > 12 {
            start.year() + yrs as i32 + 1
        } else {
            start.year() + yrs as i32
        };
        let final_day = std::cmp::min(start.day(), last_day_of_month(final_yr, final_mth));

        let end = NaiveDate::from_ymd_opt(final_yr, final_mth, final_day)
            .expect("invalid end date")
            .and_time(
                chrono::NaiveTime::from_hms_opt(start.hour(), start.minute(), start.second())
                    .expect("invalid end time"),
            );

        let now = chrono::offset::Local::now();

        let elapsed = now.timestamp() - start.timestamp();
        let total = end.and_utc().timestamp() - start.timestamp();

        let percent = elapsed as f32 / total as f32 * 100.0;

        let elapsed_days = elapsed / 60 / 60 / 24;
        let total_days = total / 60 / 60 / 24;

        format!("{elapsed_days} Days / {total_days} Days ({percent:.2}%)")
    }

    fn challenge_title(&self) -> String {
        let mut segments: Vec<String> = Vec::new();
        segments.push(if self.args.challenge_years == 0 {
            String::new()
        } else {
            format!("{} YEAR", self.args.challenge_years)
        });

        segments.push(if self.args.challenge_months == 0 {
            String::new()
        } else {
            format!("{} MONTH", self.args.challenge_months)
        });

        segments.push(match &self.args.challenge_type {
            None => String::new(),
            Some(t) => t.to_owned().to_uppercase(),
        });

        let title = segments
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        format!("  {title} CHALLENGE  ")
    }

    fn challenge_module(&self) -> Vec<serde_json::Value> {
        let title = self.challenge_title();
        let body = self.challenge_text();
        let maxlen = std::cmp::max(title.len(), body.len());

        let title = json!({
            "type": "custom",
            "format": term_color(3, &format!("{title:^maxlen$}"), true),
        });
        let sep = json!({
            "type": "custom",
            // fill line with box drawing dash
            "format": term_color(3, &format!("{:─^maxlen$}", ""), false),
        });
        let body = json!({
            "type": "custom",
            "format": body,
        });

        vec![json!("break"), title, sep, body]
    }

    pub fn create_config(&self, config_jsonc: &PathBuf) {
        let kernel = json!({ "type": "kernel", "key": " VER", });
        let uptime = json!({ "type": "uptime", "key": "󰅐 UP", });
        let packages = json!({ "type": "packages", "key": "󰏖 PKG", });
        let display = json!({ "type": "display", "key": "󰍹 RES", "compactType": "scaled" });
        let terminal = json!({ "type": "terminal", "key": " TER", "format": "{3}" });
        let cpu = json!({ "type": "cpu", "key": " CPU", "format": "{1} ({5})", });
        let memory =
            json!({ "type": "memory", "key": "󰆼 RAM", "format": "{/1}{-}{/}{/2}{-}{/}{} / {}" });
        let color = json!({ "type": "colors", "symbol": "circle", });

        let mut modules = vec![
            self.os_module(),
            kernel,
            uptime,
            packages,
            json!("break"),
            cpu,
        ];
        // might have multiple gpus
        modules.extend(self.gpu_module());
        modules.extend([
            memory,
            json!("break"),
            display,
            self.wm_module(),
            terminal,
            self.shell_module(),
        ]);

        // set colors for modules
        if !self.args.no_color_keys {
            let colors = ["green", "yellow", "blue", "magenta", "cyan"];
            for (i, module) in modules.iter_mut().enumerate() {
                if let Value::Object(module) = module {
                    module.insert("keyColor".into(), json!(colors[i % colors.len()]));
                }
            }
        }

        // optional challenge block
        if self.args.challenge {
            modules.extend_from_slice(&self.challenge_module());
        }

        modules.extend([json!("break"), color]);

        let contents = json!( {
            "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
            "display": {
                "separator": "   ",
                // icon + space + 3 letters + separator
                "key": {
                    "width": 1 + 1 + 3 + 3,
                },
                "size": {
                    "binaryPrefix": "si",
                },
            },
            "logo": self.logo_module(),
            "modules": modules,
        });

        // write json to file
        let file = std::fs::File::create(config_jsonc)
            .unwrap_or_else(|_| panic!("failed to create json config"));
        serde_json::to_writer(file, &contents)
            .unwrap_or_else(|_| panic!("failed to write json config"));
    }
}

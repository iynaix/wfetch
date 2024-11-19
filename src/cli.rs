use clap::{ArgGroup, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};

#[allow(clippy::struct_excessive_bools)]
#[derive(Parser, Debug, Clone)]
#[command(
    name = "wfetch",
    about = "wfetch is an opinionated command-line fetch tool for displaying system information in a pretty way",
    version = env!("CARGO_PKG_VERSION"),
)]
// only one type of logo is allowed
#[command(group = ArgGroup::new("logo")
    .args(&[
        #[cfg(feature = "nixos")]
        "hollow",
        #[cfg(feature = "nixos")]
        "waifu",
        #[cfg(feature = "nixos")]
        "waifu2",
        "wallpaper",
        "wallpaper_ascii"
    ])
    .multiple(false))]
#[command(group = ArgGroup::new("image_options").args(&[
    "wallpaper",
    #[cfg(feature = "nixos")]
    "waifu",
    #[cfg(feature = "nixos")]
    "waifu2"
]))]
pub struct WFetchArgs {
    #[cfg(feature = "nixos")]
    #[arg(long, action, help = "Show hollow NixOS logo")]
    pub hollow: bool,

    #[cfg(feature = "nixos")]
    #[arg(
        long,
        action,
        alias = "waifu1",
        help = "Show waifu NixOS logo with dynamic colors"
    )]
    pub waifu: bool,

    #[cfg(feature = "nixos")]
    #[arg(long, action, help = "Show waifu NixOS logo 2 with dynamic colors")]
    pub waifu2: bool,

    #[arg(
        long,
        num_args = 0..=1,
        value_name = "WALLPAPER",
        default_missing_value = "",
        action,
        help = "Show section of wallpaper",
        long_help = "Show section of wallpaper, use \"-\" for stdin\n(supported backends: swww, swaybg, hyprpaper, gnome, cinnamon, mate)",
    )]
    pub wallpaper: Option<String>,

    #[arg(
        long,
        num_args = 0..=1,
        visible_aliases = ["ascii-wallpaper", "ascii"],
        value_name = "WALLPAPER",
        default_missing_value = "",
        action,
        help = "Show section of wallpaper in ascii",
        long_help = "Show section of wallpaper in ascii, use \"-\" for stdin\n(supported backends: swww, swaybg, hyprpaper, gnome, cinnamon, mate)",
    )]
    pub wallpaper_ascii: Option<String>,

    #[arg(long, action, help = "Show challenge progress")]
    pub challenge: bool,

    #[arg(
        long,
        action,
        value_name = "TIMESTAMP",
        default_value = "1675821503",
        help = "Start of the challenge as a UNIX timestamp in seconds"
    )]
    pub challenge_timestamp: i32,

    #[arg(
        long,
        action,
        value_name = "YEARS",
        default_value = "10",
        help = "Duration of challenge in years"
    )]
    pub challenge_years: u32,

    #[arg(
        long,
        action,
        value_name = "MONTHS",
        default_value = "0",
        help = "Duration of challenge in months"
    )]
    pub challenge_months: u32,

    #[arg(long, action, help = "Type of the challenge, e.g. emacs")]
    pub challenge_type: Option<String>,

    #[arg(long, action, visible_aliases=["socket"], help = "Listen for SIGUSR2 to refresh output")]
    pub listen: bool,

    #[arg(long, action, help = "Do not show colored keys")]
    pub no_color_keys: bool,

    #[arg(
        long,
        action,
        help = "Image size in pixels",
        requires = "image_options"
    )]
    pub image_size: Option<u32>,

    #[arg(
        long,
        action,
        default_value = "70",
        help = "Ascii size in characters",
        requires = "wallpaper_ascii"
    )]
    pub ascii_size: i32,

    #[arg(long, action, help = "Scale factor for high DPI displays")]
    pub scale: Option<f64>,

    #[arg(
        long,
        value_enum,
        help = "Type of shell completion to generate",
        hide = true,
        exclusive = true
    )]
    pub generate: Option<ShellCompletion>,
}

#[derive(Subcommand, ValueEnum, Debug, Clone)]
pub enum ShellCompletion {
    Bash,
    Zsh,
    Fish,
}

pub fn generate_completions(shell_completion: &ShellCompletion) {
    let mut cmd = WFetchArgs::command();

    match shell_completion {
        ShellCompletion::Bash => generate(Shell::Bash, &mut cmd, "focal", &mut std::io::stdout()),
        ShellCompletion::Zsh => generate(Shell::Zsh, &mut cmd, "focal", &mut std::io::stdout()),
        ShellCompletion::Fish => generate(Shell::Fish, &mut cmd, "focal", &mut std::io::stdout()),
    }
}

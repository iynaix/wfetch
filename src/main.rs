use clap::Parser;
use color_eyre::eyre::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    terminal::{Clear, ClearType},
};
use signal_hook::{
    consts::{SIGINT, SIGUSR2},
    iterator::Signals,
};
use std::{
    io::stdout,
    process::{Command, Output, Stdio},
    thread,
    time::Duration,
};
use wfetch::{
    Fastfetch,
    cli::{WFetchArgs, generate_completions},
    create_output_file,
};

fn wfetch(args: &WFetchArgs) -> std::io::Result<Output> {
    let config_jsonc = create_output_file("wfetch.jsonc");

    Fastfetch::new(args).create_config(&config_jsonc);

    Command::new("fastfetch")
        .arg("--hide-cursor")
        .arg("--config")
        .arg(config_jsonc)
        .stdout(Stdio::inherit())
        .output()
}

fn main() -> Result<()> {
    let args = WFetchArgs::parse();

    // print shell completions
    if let Some(shell) = args.generate {
        return generate_completions(&shell);
    }

    crossterm::execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;

    // initial display of wfetch
    wfetch(&args)?;

    // not showing waifu / wallpaper, no need to wait for signal
    if !args.listen {
        return Ok(());
    }

    crossterm::execute!(stdout(), Hide)?;

    // handle SIGUSR2 to update colors
    // https://rust-cli.github.io/book/in-depth/signals.html#handling-other-types-of-signals
    let mut signals = Signals::new([SIGINT, SIGUSR2])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    crossterm::execute!(stdout(), Show).ok();
                    std::process::exit(0);
                }
                SIGUSR2 => {
                    wfetch(&args).ok();
                }
                _ => unreachable!(),
            }
        }
    });

    loop {
        thread::sleep(Duration::from_millis(200));
    }
}

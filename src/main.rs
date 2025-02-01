use clap::Parser;
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
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use wfetch::{
    cli::{generate_completions, WFetchArgs},
    create_output_file, Fastfetch,
};

fn wfetch(args: &WFetchArgs) {
    let config_jsonc = create_output_file("wfetch.jsonc");

    Fastfetch::new(args).create_config(&config_jsonc);

    Command::new("fastfetch")
        .arg("--hide-cursor")
        .arg("--config")
        .arg(config_jsonc)
        .stdout(Stdio::inherit())
        .output()
        .expect("failed to run fastfetch");
}

fn main() {
    let args = WFetchArgs::parse();

    // print shell completions
    if let Some(shell) = args.generate {
        return generate_completions(&shell);
    }

    crossterm::execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))
        .expect("Failed to clear screen");

    // initial display of wfetch
    wfetch(&args);

    // not showing waifu / wallpaper, no need to wait for signal
    if !args.listen {
        return;
    }

    crossterm::execute!(stdout(), Hide).expect("Failed to hide cursor");

    // handle SIGUSR2 to update colors
    // https://rust-cli.github.io/book/in-depth/signals.html#handling-other-types-of-signals
    let mut signals = Signals::new([SIGINT, SIGUSR2]).expect("failed to register signals");

    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    crossterm::execute!(stdout(), Show).expect("Failed to restore cursor");
                    std::process::exit(0);
                }
                SIGUSR2 => {
                    wfetch(&args);
                }
                _ => unreachable!(),
            }
        }
    });

    loop {
        thread::sleep(Duration::from_millis(200));
    }
}

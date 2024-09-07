use clap::Parser;
use execute::Execute;
use signal_hook::{
    consts::{SIGINT, SIGUSR2},
    iterator::Signals,
};
use std::{
    io::{self, Write},
    thread,
    time::Duration,
};
use wfetch::{
    cli::{generate_completions, WFetchArgs},
    create_fastfetch_config, create_output_file, show_wallpaper_ascii,
};

fn wfetch(args: &WFetchArgs) {
    let config_jsonc = create_output_file("wfetch.jsonc");
    create_fastfetch_config(args, &config_jsonc);

    let mut fastfetch =
        execute::command_args!("fastfetch", "--hide-cursor", "--config", config_jsonc);

    if args.wallpaper_ascii.is_some() {
        // clear screen
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().expect("Failed to flush stdout");

        show_wallpaper_ascii(args, &mut fastfetch);
    } else {
        fastfetch
            .execute_output()
            .expect("failed to execute fastfetch");
    }
}

fn main() {
    let args = WFetchArgs::parse();

    if args.version {
        println!("wfetch {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    // print shell completions
    if let Some(shell) = args.generate {
        return generate_completions(&shell);
    }

    // clear screen
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().expect("Failed to clear screen");

    // initial display of wfetch
    wfetch(&args);

    // not showing waifu / wallpaper, no need to wait for signal
    if !args.listen {
        return;
    }

    // hide terminal cursor
    print!("\x1B[?25l");
    io::stdout()
        .flush()
        .expect("Failed to hide terminal cursor");

    // handle SIGUSR2 to update colors
    // https://rust-cli.github.io/book/in-depth/signals.html#handling-other-types-of-signals
    let mut signals = Signals::new([SIGINT, SIGUSR2]).expect("failed to register signals");

    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    // restore terminal cursor
                    print!("\x1B[?25h");
                    io::stdout()
                        .flush()
                        .expect("Failed to restore terminal cursor");
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

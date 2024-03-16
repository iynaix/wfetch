use clap::{Command, CommandFactory};
use clap_complete::{
    generate_to,
    shells::{Bash, Fish, Zsh},
};

include!("src/cli.rs");

pub fn generate_completions(mut cli: Command) -> Result<(), std::io::Error> {
    let cmd_name = cli.get_name().to_string();
    let out = "completions";

    std::fs::create_dir_all(out)?;
    generate_to(Bash, &mut cli, &cmd_name, out)?;
    generate_to(Zsh, &mut cli, &cmd_name, out)?;
    generate_to(Fish, &mut cli, &cmd_name, out)?;

    Ok(())
}

fn main() -> Result<(), std::io::Error> {
    generate_completions(WFetchArgs::command())?;

    // override with the version passed in from nix
    // https://github.com/rust-lang/cargo/issues/6583#issuecomment-1259871885
    if let Ok(val) = std::env::var("NIX_RELEASE_VERSION") {
        println!("cargo:rustc-env=CARGO_PKG_VERSION={val}");
    }
    println!("cargo:rerun-if-env-changed=NIX_RELEASE_VERSION");

    Ok(())
}

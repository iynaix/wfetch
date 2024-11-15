use clap::CommandFactory;
use clap_mangen::Man;
use std::{fs, path::PathBuf};

#[allow(dead_code)]
#[path = "src/cli.rs"]
mod cli;

fn generate_man_pages() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = cli::WFetchArgs::command();
    let man_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/man");
    fs::create_dir_all(&man_dir)?;

    let mut buffer = Vec::default();
    Man::new(cmd).render(&mut buffer)?;
    fs::write(man_dir.join("wfetch.1"), buffer)?;

    Ok(())
}

fn main() {
    // override with the version passed in from nix
    // https://github.com/rust-lang/cargo/issues/6583#issuecomment-1259871885
    if let Ok(val) = std::env::var("NIX_RELEASE_VERSION") {
        println!("cargo:rustc-env=CARGO_PKG_VERSION={val}");
    }
    println!("cargo:rerun-if-env-changed=NIX_RELEASE_VERSION");

    // generate man pages
    if let Err(err) = generate_man_pages() {
        println!("cargo:warning=Error generating man pages: {err}");
    }
}

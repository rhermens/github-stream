#[path = "src/cli.rs"]
mod cli;

use clap::CommandFactory;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=src/cli.rs");

    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo");
    clap_mangen::Man::new(cli::Args::command()).generate_to(out_dir)?;

    Ok(())
}

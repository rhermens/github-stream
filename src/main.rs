use std::{thread, time::Duration};

use clap::Parser;

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long)]
    pub interval: u8,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let octocrab = octocrab::instance();

    loop {
        println!("{:?} {:?}", octocrab, args);
        thread::sleep(Duration::from_secs(args.interval.into()));
    }
}

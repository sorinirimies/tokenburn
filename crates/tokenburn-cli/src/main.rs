use std::io;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = tokenburn_cli::Args::parse();
    tokenburn_cli::run(&args, &mut io::stdout().lock())
}

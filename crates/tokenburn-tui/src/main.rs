use clap::Parser;
use tokenburn_tui::Args;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    tokenburn_tui::run(args.query())
}

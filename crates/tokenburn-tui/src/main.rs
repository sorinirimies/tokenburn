use clap::Parser;
use tokenburn_tui::Args;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.list_themes {
        for name in tokenburn_core::THEME_NAMES {
            println!("{name}");
        }
        return Ok(());
    }
    tokenburn_tui::run(args.query(), args.theme)
}

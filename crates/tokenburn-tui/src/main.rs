use clap::Parser;
use tokenburn_tui::Args;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.list_themes {
        println!("{}", tokenburn_core::SYSTEM_THEME_NAME);
        for name in tokenburn_core::THEME_NAMES {
            println!("{name}");
        }
        return Ok(());
    }
    if args.list_currencies {
        for c in tokenburn_core::CURRENCIES {
            println!("{:<4}{}", c.code, c.name);
        }
        return Ok(());
    }
    tokenburn_tui::run(args.query(), args.theme, args.currency)
}

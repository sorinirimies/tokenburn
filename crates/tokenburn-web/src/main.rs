//! `tokenburn-web` — local web dashboard.
//!
//! Binds `127.0.0.1:3000` by default; `--host` / `--port` (or the `HOST` /
//! `PORT` env vars) override. The page opens in your browser on start;
//! pass `--no-open` to skip.

use clap::Parser;
use tokenburn_web::{ServerOptions, serve};

#[derive(Debug, Parser)]
#[command(name = "tokenburn-web", version, about)]
struct Args {
    /// Interface to bind (use 0.0.0.0 to expose on the network — there is no auth).
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on.
    #[arg(long, short, env = "PORT", default_value_t = 3000)]
    port: u16,

    /// Don't open the dashboard in a browser on start.
    #[arg(long)]
    no_open: bool,

    /// Colour theme, or "System" to follow the OS light/dark setting (see --list-themes); remembered for the TUI and GUI too.
    #[arg(long, value_name = "NAME", value_parser = parse_theme)]
    theme: Option<tokenburn_core::ThemeChoice>,

    /// Print every theme name and exit.
    #[arg(long)]
    list_themes: bool,
}

fn parse_theme(s: &str) -> Result<tokenburn_core::ThemeChoice, String> {
    tokenburn_core::ThemeChoice::parse(s)
        .ok_or_else(|| format!("unknown theme '{s}' (try --list-themes)"))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    if args.list_themes {
        println!("{}", tokenburn_core::SYSTEM_THEME_NAME);
        for name in tokenburn_core::THEME_NAMES {
            println!("{name}");
        }
        return Ok(());
    }
    match args.theme {
        Some(tokenburn_core::ThemeChoice::System) => {
            let _ = tokenburn_core::settings::save_system_theme();
        }
        Some(tokenburn_core::ThemeChoice::Preset(i)) => {
            let _ = tokenburn_core::settings::save_theme(i);
        }
        None => {}
    }
    serve(ServerOptions {
        host: args.host,
        port: args.port,
        open_browser: !args.no_open,
    })
    .await
}

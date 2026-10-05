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
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    serve(ServerOptions {
        host: args.host,
        port: args.port,
        open_browser: !args.no_open,
    })
    .await
}

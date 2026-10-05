//! Server setup: options, router construction and the serve loop.

use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::{Cx, app_context},
    router::{Router, RouterBuilderDiscoverExt},
    runtime::RouterBuilderRuntimeExt,
};

/// Where and how to serve.
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Interface to bind. Defaults to `127.0.0.1`: token/cost data is private.
    pub host: String,
    pub port: u16,
    /// Open the dashboard in the default browser once the server is listening.
    pub open_browser: bool,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3000,
            open_browser: false,
        }
    }
}

/// Whether the browser runtime (signals/shards/WebSocket) is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interactive(pub bool);

/// Read the [`Interactive`] flag from the router's app context.
pub(crate) fn is_interactive(cx: &Cx) -> bool {
    app_context::<Interactive>(cx).0
}

/// Build the router. `Some(bundle)` enables live mode; `None` serves the
/// static dashboard.
pub fn router(bundle: Option<AssetBundle>) -> Router {
    let builder = Router::builder().discover();
    match bundle {
        Some(bundle) => builder
            .app_context(Interactive(true))
            .assets(bundle)
            .runtime()
            .build(),
        None => builder.app_context(Interactive(false)).build(),
    }
}

/// Bind and serve until the process is killed.
pub async fn serve(opts: ServerOptions) -> std::io::Result<()> {
    let bundle = match AssetBundle::load() {
        Ok(b) => Some(b),
        Err(e) => {
            eprintln!(
                "tokenburn-web: no asset bundle ({e}); serving the static dashboard.\n\
                 Run `just run-web` (or `topcoat asset bundle` first) for the live version."
            );
            None
        }
    };
    let live = bundle.is_some();

    let listener = tokio::net::TcpListener::bind((opts.host.as_str(), opts.port)).await?;
    let addr = listener.local_addr()?;
    println!(
        "tokenburn-web listening on http://{addr} ({})",
        if live { "live" } else { "static" }
    );
    if !addr.ip().is_loopback() {
        eprintln!(
            "tokenburn-web: warning — bound to {addr}; there is no auth, anyone who can reach it sees your token usage."
        );
    }
    if opts.open_browser {
        // The listener is already bound, so the browser can connect right away.
        let url = browser_url(addr);
        if let Err(e) = open_in_browser(&url) {
            eprintln!("tokenburn-web: could not open a browser ({e}); visit {url}");
        }
    }
    topcoat::serve(listener, router(bundle)).await
}

/// URL to open for a bound address (`0.0.0.0` / `::` are not browsable → localhost).
pub fn browser_url(addr: std::net::SocketAddr) -> String {
    if addr.ip().is_unspecified() {
        format!("http://localhost:{}", addr.port())
    } else if addr.is_ipv6() {
        format!("http://[{}]:{}", addr.ip(), addr.port())
    } else {
        format!("http://{addr}")
    }
}

/// Open `url` with the platform's default handler (no extra dependency).
fn open_in_browser(url: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};

    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut cmd = Command::new("xdg-open");

    cmd.arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unspecified_addresses_open_localhost() {
        assert_eq!(
            browser_url("0.0.0.0:3000".parse().unwrap()),
            "http://localhost:3000"
        );
        assert_eq!(
            browser_url("[::]:8080".parse().unwrap()),
            "http://localhost:8080"
        );
    }

    #[test]
    fn concrete_addresses_are_used_as_is() {
        assert_eq!(
            browser_url("127.0.0.1:3000".parse().unwrap()),
            "http://127.0.0.1:3000"
        );
        assert_eq!(
            browser_url("[::1]:3000".parse().unwrap()),
            "http://[::1]:3000"
        );
    }
}

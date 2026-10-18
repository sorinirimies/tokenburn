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

/// Directories searched for the Topcoat asset bundle (the browser runtime),
/// in order. `exe_dir` is the directory of the running executable.
///
/// 1. `$TOKENBURN_WEB_ASSETS`
/// 2. `<exe_dir>/assets` — release archives, `cargo run`, Windows installs
/// 3. `<exe_dir>/../share/tokenburn-web/assets` — Homebrew / prefix installs
/// 4. `/usr/share/tokenburn-web/assets`, `/usr/local/share/tokenburn-web/assets` — .deb/.rpm/AUR
pub fn bundle_candidates(
    exe_dir: Option<&std::path::Path>,
    env_override: Option<std::ffi::OsString>,
) -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;
    let mut out: Vec<PathBuf> = Vec::new();
    out.extend(env_override.filter(|v| !v.is_empty()).map(PathBuf::from));
    if let Some(dir) = exe_dir {
        out.push(dir.join("assets"));
        out.push(dir.join("../share/tokenburn-web/assets"));
    }
    out.push(PathBuf::from("/usr/share/tokenburn-web/assets"));
    out.push(PathBuf::from("/usr/local/share/tokenburn-web/assets"));
    out
}

/// Find and load the asset bundle, or `None` (→ static dashboard).
fn find_bundle() -> Option<AssetBundle> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
    let candidates =
        bundle_candidates(exe_dir.as_deref(), std::env::var_os("TOKENBURN_WEB_ASSETS"));
    candidates
        .iter()
        .find_map(|dir| AssetBundle::load_dir(dir).ok())
}

/// Bind and serve until the process is killed.
pub async fn serve(opts: ServerOptions) -> std::io::Result<()> {
    let bundle = find_bundle();
    if bundle.is_none() {
        eprintln!(
            "tokenburn-web: no asset bundle found; serving the static dashboard.\n\
             For the live version run `just run-web`, use a release archive/package, or set \
             TOKENBURN_WEB_ASSETS to a `topcoat asset bundle` directory."
        );
    }
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
    // Spending limits: look every 30 s, page open or not, and raise a desktop
    // notification when a level is crossed.
    tokio::spawn(async {
        loop {
            for a in crate::data::watch_limits().await {
                println!("tokenburn-web: {}", a.title());
            }
            tokio::time::sleep(crate::data::WATCH_EVERY).await;
        }
    });
    // Exit immediately on Ctrl+C / SIGTERM: open live-update streams would
    // otherwise hold graceful shutdown until its timeout.
    tokio::select! {
        r = topcoat::serve(listener, router(bundle)) => r,
        () = crate::lifecycle::shutdown_signal() => Ok(()),
    }
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
    fn bundle_search_order() {
        let exe = std::path::Path::new("/opt/tb");
        let c = bundle_candidates(Some(exe), Some("/custom".into()));
        assert_eq!(c[0], std::path::PathBuf::from("/custom"));
        assert_eq!(c[1], std::path::PathBuf::from("/opt/tb/assets"));
        assert!(
            c.iter()
                .any(|p| p.ends_with("usr/share/tokenburn-web/assets"))
        );
    }

    #[test]
    fn empty_override_and_unknown_exe_dir_are_tolerated() {
        let c = bundle_candidates(None, Some("".into()));
        assert!(c.iter().all(|p| p.starts_with("/usr")));
    }

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

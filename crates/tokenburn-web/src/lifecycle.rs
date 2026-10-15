//! Process lifetime: exit when the last dashboard tab closes or on Ctrl+C / SIGTERM.
//!
//! Pages load `/lifecycle.js`, which beacons `hello` on load and `bye` on
//! `pagehide`. After the last tab says bye the server waits a short grace
//! period (reloads re-hello) and then exits.

use std::{
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    time::Duration,
};

use topcoat::{
    Result,
    router::{content::Js, route},
};

static OPEN: AtomicUsize = AtomicUsize::new(0);
static GENERATION: AtomicU64 = AtomicU64::new(0);

const GRACE: Duration = Duration::from_secs(3);

const JS: &str = r#"(function(){
  var open=false;
  function hello(){ if(!open){ open=true; navigator.sendBeacon('/api/hello'); } }
  function bye(){ if(open){ open=false; navigator.sendBeacon('/api/bye'); } }
  hello();
  addEventListener('pagehide', bye);
  addEventListener('pageshow', hello);
})();"#;

#[route(GET "/lifecycle.js")]
async fn script() -> Result<Js<&'static str>> {
    Ok(Js(JS))
}

#[route(POST "/api/hello")]
async fn hello() -> Result<()> {
    OPEN.fetch_add(1, Ordering::SeqCst);
    GENERATION.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

#[route(POST "/api/bye")]
async fn bye() -> Result<()> {
    let _ = OPEN.try_update(Ordering::SeqCst, Ordering::SeqCst, |n| Some(n.saturating_sub(1)));
    let generation = GENERATION.load(Ordering::SeqCst);
    tokio::spawn(async move {
        tokio::time::sleep(GRACE).await;
        if OPEN.load(Ordering::SeqCst) == 0 && GENERATION.load(Ordering::SeqCst) == generation {
            println!("tokenburn-web: last page closed, exiting");
            std::process::exit(0);
        }
    });
    Ok(())
}

/// Resolves on Ctrl+C or SIGTERM.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { () = ctrl_c => {}, () = term => {} }
}

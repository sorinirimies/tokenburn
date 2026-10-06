//! End-to-end tests of the static dashboard: a real server on an ephemeral port,
//! driven with plain HTTP/1.1 over a `TcpStream` (no client dependency).
//!
//! The environment is pointed at a throw-away sandbox *before* anything reads
//! it, so these tests never see (or write) the developer's real logs/settings.

use std::{net::SocketAddr, path::PathBuf, sync::OnceLock};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct Sandbox {
    config: PathBuf,
}

fn sandbox() -> &'static Sandbox {
    static BOX: OnceLock<Sandbox> = OnceLock::new();
    BOX.get_or_init(|| {
        let root = std::env::temp_dir().join(format!("tokenburn-web-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let sessions = root.join("sessions");
        let proj = sessions.join("--demo-proj--");
        std::fs::create_dir_all(&proj).unwrap();
        let line = |ts: &str, i: u64| {
            format!(
                r#"{{"timestamp":"{ts}","message":{{"usage":{{"input":{i},"output":{},"cacheRead":0,"cacheWrite":0,"cost":{{"total":0.5}}}}}}}}"#,
                i / 2
            )
        };
        let body = [
            line("2026-05-01T10:00:00Z", 1000),
            line("2026-05-02T10:00:00Z", 2000),
            line("2026-05-02T11:00:00Z", 3000),
        ]
        .join("\n");
        std::fs::write(proj.join("a.jsonl"), body).unwrap();
        let config = root.join("config");
        // SAFETY: runs once, inside `get_or_init`, before any server thread reads
        // the environment (every test calls `sandbox()` first).
        unsafe {
            std::env::set_var("HOME", &root);
            std::env::set_var("USERPROFILE", &root);
            std::env::set_var("XDG_DATA_HOME", root.join("xdg"));
            std::env::set_var("XDG_CONFIG_HOME", root.join("xdg-config"));
            std::env::set_var("APPDATA", root.join("appdata"));
            std::env::set_var("LOCALAPPDATA", root.join("localappdata"));
            std::env::set_var("TOKENBURN_PI_SESSIONS", &sessions);
            std::env::set_var("TOKENBURN_ZED_DB", root.join("none.db"));
            std::env::set_var("TOKENBURN_CONFIG_DIR", &config);
            for v in [
                "PI_CODING_AGENT_SESSION_DIR",
                "PI_CODING_AGENT_DIR",
                "CLAUDE_CONFIG_DIR",
                "CODEX_HOME",
                "COPILOT_HOME",
                "GEMINI_DATA_DIR",
                "OPENCODE_DATA_DIR",
                "AMP_DATA_DIR",
            ] {
                std::env::remove_var(v);
            }
        }
        Sandbox { config }
    })
}

async fn start() -> SocketAddr {
    sandbox();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = topcoat::serve(listener, tokenburn_web::router(None)).await;
    });
    addr
}

struct Response {
    status: u16,
    headers: String,
    body: String,
}

fn dechunk(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(nl) = rest.windows(2).position(|w| w == b"\r\n") {
        let size = usize::from_str_radix(std::str::from_utf8(&rest[..nl]).unwrap().trim(), 16)
            .unwrap_or(0);
        if size == 0 {
            break;
        }
        out.extend_from_slice(&rest[nl + 2..nl + 2 + size]);
        rest = &rest[nl + 2 + size + 2..];
    }
    out
}

async fn get(addr: SocketAddr, path: &str) -> Response {
    let mut s = TcpStream::connect(addr).await.unwrap();
    s.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .await
    .unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).await.unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    let chunked = head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked");
    let body = &raw[split + 4..];
    let body = if chunked {
        dechunk(body)
    } else {
        body.to_vec()
    };
    Response {
        status,
        headers: head.to_ascii_lowercase(),
        body: String::from_utf8_lossy(&body).to_string(),
    }
}

const ALL_DAILY: &str = "tool=pi&bucket=daily&window=all";

#[tokio::test]
async fn health_and_static_assets() {
    let addr = start().await;
    let h = get(addr, "/api/health").await;
    assert_eq!(h.status, 200);
    assert_eq!(h.body.trim(), "\"ok\"");

    let css = get(addr, "/style.css").await;
    assert_eq!(css.status, 200);
    assert!(css.headers.contains("text/css"));
    assert!(css.body.contains(".charts"), "chart styles are served");

    assert_eq!(get(addr, "/nope").await.status, 404);
}

#[tokio::test]
async fn the_dashboard_renders_summary_charts_and_table() {
    let addr = start().await;
    let r = get(addr, &format!("/?{ALL_DAILY}")).await;
    assert_eq!(r.status, 200);
    assert!(r.headers.contains("text/html"));
    let html = r.body;
    assert!(html.contains("TokenBurn"));
    assert!(
        html.contains("http-equiv=\"refresh\""),
        "static mode reloads itself"
    );
    // Summary: 3 turns, 6000 input.
    assert!(html.contains(">3<") || html.contains("3\n"), "turn count");
    // Charts.
    assert_eq!(
        html.matches("<svg class=\"chart").count(),
        2,
        "bars + donut"
    );
    assert!(html.contains("Tokens over time"));
    assert!(html.contains("Share by project"));
    assert!(html.contains("<rect"), "bar segments");
    assert!(html.contains("<title>"), "hover tooltips");
    // Table rows, one per day.
    assert!(html.contains("2026-05-01") && html.contains("2026-05-02"));
    assert!(html.contains("demo-proj") || html.contains("pi"));
}

#[tokio::test]
async fn the_chart_metric_switches_with_the_query() {
    let addr = start().await;
    let r = get(addr, &format!("/?{ALL_DAILY}&metric=cost")).await;
    assert!(r.body.contains("Cost over time"));
    assert!(r.body.contains("selected") && r.body.contains("cost"));
    let r = get(addr, &format!("/?{ALL_DAILY}&metric=garbage")).await;
    assert!(
        r.body.contains("Tokens over time"),
        "unknown metric → tokens"
    );
}

#[tokio::test]
async fn an_empty_selection_says_so_instead_of_breaking() {
    let addr = start().await;
    let r = get(addr, "/?tool=amp&bucket=daily&window=all").await;
    assert_eq!(r.status, 200);
    assert!(r.body.contains("No data"), "empty charts show a message");
}

#[tokio::test]
async fn untrusted_query_values_are_ignored_and_never_reflected() {
    let addr = start().await;
    let r = get(
        addr,
        "/?tool=%3Cscript%3Ealert(1)%3C/script%3E&bucket=x&window=y",
    )
    .await;
    assert_eq!(r.status, 200);
    assert!(!r.body.contains("<script>alert"), "no reflected markup");
}

#[tokio::test]
async fn the_json_api_returns_the_same_report() {
    let addr = start().await;
    let r = get(addr, &format!("/api/report?{ALL_DAILY}")).await;
    assert_eq!(r.status, 200);
    assert!(r.headers.contains("application/json"));
    let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
    let rows = v.as_array().unwrap();
    assert_eq!(rows.len(), 2, "two days");
    let total: u64 = rows
        .iter()
        .map(|r| r["total_tokens"].as_u64().unwrap())
        .sum();
    assert_eq!(total, 1000 + 500 + 2000 + 1000 + 3000 + 1500);
}

#[tokio::test]
async fn themes_are_listed_and_a_chosen_one_is_applied_and_remembered() {
    let addr = start().await;
    let t = get(addr, "/api/themes").await;
    let v: serde_json::Value = serde_json::from_str(&t.body).unwrap();
    let n = v
        .as_array()
        .map_or_else(|| v["themes"].as_array().unwrap().len(), Vec::len);
    assert_eq!(n, tokenburn_core::THEME_COUNT);

    let r = get(addr, "/?theme=Dracula&window=all").await;
    assert!(r.body.contains("--s0"), "theme variables are inlined");
    let saved = std::fs::read_to_string(sandbox().config.join("settings.json")).unwrap();
    assert!(saved.contains("Dracula"), "{saved}");

    // An unknown theme keeps the saved one.
    let r = get(addr, "/?theme=Nope&window=all").await;
    assert_eq!(r.status, 200);
    let saved = std::fs::read_to_string(sandbox().config.join("settings.json")).unwrap();
    assert!(saved.contains("Dracula"));
}

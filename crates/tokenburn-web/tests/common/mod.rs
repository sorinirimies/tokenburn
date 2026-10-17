//! Shared harness for the web end-to-end tests: a sandboxed environment and a tiny
//! HTTP client. Each test *binary* gets its own sandbox (its own process), so tests
//! that change saved settings can live in their own file without racing the others.
#![allow(dead_code)]

use std::{net::SocketAddr, path::PathBuf, sync::OnceLock};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub struct Sandbox {
    pub config: PathBuf,
}

pub fn sandbox() -> &'static Sandbox {
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
        // An rtk-shaped history database: 2M tokens saved by 40 commands.
        let rtk = rusqlite::Connection::open(root.join("rtk.db")).unwrap();
        rtk.execute_batch(
            "CREATE TABLE commands (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, \
             original_cmd TEXT, rtk_cmd TEXT, input_tokens INTEGER, output_tokens INTEGER, \
             saved_tokens INTEGER NOT NULL, savings_pct REAL);",
        )
        .unwrap();
        for i in 0..40 {
            rtk.execute(
                "INSERT INTO commands (timestamp, saved_tokens) VALUES (?1, 50000)",
                [format!("2026-05-01T10:{i:02}:00+00:00")],
            )
            .unwrap();
        }
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
            std::env::set_var("TOKENBURN_RTK_DB", root.join("rtk.db"));
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

pub async fn start() -> SocketAddr {
    sandbox();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = topcoat::serve(listener, tokenburn_web::router(None)).await;
    });
    addr
}

pub struct Response {
    pub status: u16,
    pub headers: String,
    pub body: String,
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

pub async fn get(addr: SocketAddr, path: &str) -> Response {
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

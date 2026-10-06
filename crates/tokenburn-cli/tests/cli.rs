//! Runs the real `tokenburn` binary against a sandbox, so nothing here can see
//! the developer's logs or touch their settings (each child gets its own env).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sandbox(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("tokenburn-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let proj = root.join("sessions/--demo--");
    std::fs::create_dir_all(&proj).unwrap();
    let line = |ts: &str, input: u64| {
        format!(
            r#"{{"timestamp":"{ts}","message":{{"usage":{{"input":{input},"output":10,"cacheRead":0,"cacheWrite":0,"cost":{{"total":0.25}}}}}}}}"#
        )
    };
    std::fs::write(
        proj.join("a.jsonl"),
        [
            line("2026-05-01T10:00:00Z", 100),
            line("2026-05-01T11:00:00Z", 200),
            line("2026-06-03T09:00:00Z", 300),
        ]
        .join("\n"),
    )
    .unwrap();
    root
}

fn tokenburn(root: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tokenburn"));
    cmd.args(args)
        .env("HOME", root)
        .env("USERPROFILE", root)
        .env("XDG_DATA_HOME", root.join("xdg"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("APPDATA", root.join("appdata"))
        .env("LOCALAPPDATA", root.join("localappdata"))
        .env("TOKENBURN_PI_SESSIONS", root.join("sessions"))
        .env("TOKENBURN_ZED_DB", root.join("none.db"))
        .env("TOKENBURN_CONFIG_DIR", root.join("config"));
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
        cmd.env_remove(v);
    }
    cmd.output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

#[test]
fn prints_a_table_for_the_chosen_window() {
    let root = sandbox("table");
    let o = tokenburn(&root, &["--tool", "pi", "--daily", "--all"]);
    assert!(o.status.success(), "{o:?}");
    let out = stdout(&o);
    assert!(
        out.contains("2026-05-01") && out.contains("2026-06-03"),
        "{out}"
    );
    assert!(out.contains("pi"));
}

#[test]
fn json_output_parses_and_adds_up() {
    let root = sandbox("json");
    let o = tokenburn(&root, &["--tool", "pi", "--monthly", "--json"]);
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
    let rows = v.as_array().unwrap();
    assert_eq!(rows.len(), 2, "May and June");
    let turns: u64 = rows.iter().map(|r| r["turns"].as_u64().unwrap()).sum();
    assert_eq!(turns, 3);
    let cost: f64 = rows.iter().map(|r| r["cost_usd"].as_f64().unwrap()).sum();
    assert!((cost - 0.75).abs() < 1e-9);
}

#[test]
fn an_empty_selection_is_a_message_or_an_empty_array() {
    let root = sandbox("empty");
    let o = tokenburn(&root, &["--tool", "amp", "--all"]);
    assert!(o.status.success());
    assert!(stdout(&o).contains("No tokenburn results found for tool=amp"));
    let o = tokenburn(&root, &["--tool", "amp", "--all", "--json"]);
    assert_eq!(stdout(&o).trim(), "[]");
}

#[test]
fn bad_input_is_rejected_with_usage() {
    let root = sandbox("bad");
    let o = tokenburn(&root, &["--tool", "emacs"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("--tool"));
}

#[test]
fn help_and_version_work() {
    let root = sandbox("help");
    let o = tokenburn(&root, &["--help"]);
    let help = stdout(&o);
    for tool in ["claude", "codex", "copilot", "gemini", "opencode", "amp"] {
        assert!(help.contains(tool), "--help should list {tool}");
    }
    let o = tokenburn(&root, &["--version"]);
    assert!(stdout(&o).starts_with("tokenburn "));
}

#[test]
fn costs_can_be_shown_in_another_currency_and_json_stays_in_dollars() {
    let root = sandbox("currency");
    let usd = stdout(&tokenburn(&root, &["--tool", "pi", "--all"]));
    assert!(usd.contains("cost_usd") && usd.contains("$0.75"), "{usd}");

    let eur = stdout(&tokenburn(
        &root,
        &["--tool", "pi", "--all", "--currency", "eur"],
    ));
    assert!(
        eur.contains("cost_eur") && eur.contains("€0.69"),
        "0.75 USD at 0.92: {eur}"
    );

    let json = stdout(&tokenburn(
        &root,
        &["--tool", "pi", "--all", "--json", "-c", "EUR"],
    ));
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v[0]["cost_usd"].is_number(), "machine output stays in USD");
}

#[test]
fn the_saved_currency_is_the_default_and_the_flag_wins() {
    let root = sandbox("saved-currency");
    std::fs::create_dir_all(root.join("config")).unwrap();
    std::fs::write(
        root.join("config/settings.json"),
        r#"{"currency":"GBP","rates":{"GBP":0.5}}"#,
    )
    .unwrap();
    let out = stdout(&tokenburn(&root, &["--tool", "pi", "--all"]));
    assert!(
        out.contains("cost_gbp") && out.contains("£0.38"),
        "custom rate 0.5: {out}"
    );
    let out = stdout(&tokenburn(&root, &["--tool", "pi", "--all", "-c", "USD"]));
    assert!(out.contains("cost_usd"));
}

#[test]
fn unknown_currencies_are_rejected_and_the_list_is_printed() {
    let root = sandbox("list-currency");
    let o = tokenburn(&root, &["--currency", "zzz"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("unknown currency"));
    let list = stdout(&tokenburn(&root, &["--list-currencies"]));
    assert!(list.contains("EUR") && list.contains("Japanese yen") && list.lines().count() >= 20);
}

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
        .env("TOKENBURN_RTK_DB", root.join("rtk.db"))
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

/// An rtk-shaped history database in the sandbox, plus a cached pi turn and a compaction.
fn add_savings_sources(root: &Path) {
    let db = rusqlite::Connection::open(root.join("rtk.db")).unwrap();
    db.execute_batch(
        "CREATE TABLE commands (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, \
         original_cmd TEXT, rtk_cmd TEXT, input_tokens INTEGER, output_tokens INTEGER, \
         saved_tokens INTEGER NOT NULL, savings_pct REAL);
         INSERT INTO commands (timestamp, saved_tokens) VALUES ('2026-05-01T10:30:00+00:00', 1000000);",
    )
    .unwrap();
    let extra = [
        r#"{"timestamp":"2026-05-02T10:00:00Z","message":{"usage":{"input":1,"output":1,"cacheRead":1000000,"cacheWrite":0,"cost":{"total":0.5}}}}"#,
        r#"{"type":"compaction","timestamp":"2026-05-02T11:00:00Z","summary":"s","tokensBefore":1000000}"#,
    ];
    let f = root.join("sessions/--demo--/b.jsonl");
    std::fs::write(f, extra.join("\n")).unwrap();
}

#[test]
fn savings_are_listed_per_source_with_a_total() {
    let root = sandbox("savings");
    add_savings_sources(&root);
    let o = tokenburn(&root, &["--all", "--savings", "--input-price", "10"]);
    assert!(o.status.success(), "{o:?}");
    let out = stdout(&o);
    assert!(out.contains("saved (estimate)"), "{out}");
    // 1M cache reads × $10 × 0.9, 1M rtk × $10, ~1M compaction × $10
    assert!(
        out.contains("Prompt cache") && out.contains("$9.00"),
        "{out}"
    );
    assert!(out.contains("rtk") && out.contains("(1 commands)"), "{out}");
    assert!(
        out.contains("pi compaction") && out.contains("(1 compactions)"),
        "{out}"
    );
    assert!(out.contains("of what it would have cost"), "{out}");
}

#[test]
fn savings_json_wraps_the_report_and_is_opt_in() {
    let root = sandbox("savings-json");
    add_savings_sources(&root);
    let plain: serde_json::Value =
        serde_json::from_str(&stdout(&tokenburn(&root, &["--all", "--json"]))).unwrap();
    assert!(plain.is_array(), "no flag, no change to the JSON shape");

    let o = tokenburn(&root, &["--all", "--json", "-s", "--input-price", "10"]);
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
    assert!(v["report"].is_array());
    let names: Vec<&str> = v["savings"]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["source"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["prompt_cache", "rtk", "pi_compaction"]);
    assert_eq!(v["savings"]["tokens"].as_u64().unwrap(), 3_000_000);
    assert!((v["savings"]["usd"].as_f64().unwrap() - 29.0).abs() < 1e-9);
}

#[test]
fn bucketed_reports_also_list_savings_per_interval() {
    let root = sandbox("savings-buckets");
    add_savings_sources(&root);
    let run = |flag: &str| {
        stdout(&tokenburn(
            &root,
            &["--all", flag, "--savings", "--input-price", "10"],
        ))
    };
    let intervals = |out: &str| -> Vec<String> {
        out.lines()
            .skip_while(|l| !l.starts_with("saved per interval"))
            .skip(1)
            .map(String::from)
            .collect()
    };

    let daily = intervals(&run("--daily"));
    assert_eq!(daily.len(), 2, "{daily:?}");
    assert!(
        daily[0].contains("2026-05-01")
            && daily[0].contains("$10.00")
            && daily[0].contains("rtk $10.00")
    );
    assert!(daily[1].contains("2026-05-02") && daily[1].contains("$19.00"));
    assert!(daily[1].contains("Prompt cache $9.00") && daily[1].contains("pi compaction $10.00"));

    // One compaction at 11:00 and one cached turn at 10:00 are different minutes and hours.
    for flag in ["--minutely", "--hourly"] {
        let rows = intervals(&run(flag));
        assert_eq!(rows.len(), 3, "{flag}: rtk, cache, compaction: {rows:?}");
    }

    let monthly = intervals(&run("--monthly"));
    assert_eq!(monthly.len(), 1);
    assert!(monthly[0].contains("2026-05") && monthly[0].contains("$29.00"));

    let total = stdout(&tokenburn(
        &root,
        &["--all", "--savings", "--input-price", "10"],
    ));
    assert!(!total.contains("saved per interval"), "no buckets, no list");
}

#[test]
fn savings_json_carries_by_bucket_only_for_bucketed_reports() {
    let root = sandbox("savings-buckets-json");
    add_savings_sources(&root);
    let json = |args: &[&str]| -> serde_json::Value {
        serde_json::from_str(&stdout(&tokenburn(&root, args))).unwrap()
    };
    let v = json(&["--all", "--daily", "--json", "-s", "--input-price", "10"]);
    let b = v["savings"]["by_bucket"].as_array().unwrap();
    assert_eq!(b.len(), 2);
    assert_eq!(b[0]["label"], "2026-05-01");
    assert!((b[1]["savings"]["usd"].as_f64().unwrap() - 19.0).abs() < 1e-9);
    let v = json(&["--all", "--json", "-s"]);
    assert!(v["savings"].get("by_bucket").is_none());
}

#[test]
fn rtk_savings_need_every_tool_and_nothing_saved_says_so() {
    let root = sandbox("savings-scope");
    add_savings_sources(&root);
    let out = stdout(&tokenburn(&root, &["--tool", "pi", "--all", "-s"]));
    assert!(
        !out.contains("(1 commands)"),
        "rtk is not attributable to pi: {out}"
    );

    let bare = sandbox("savings-none");
    let out = stdout(&tokenburn(&bare, &["--all", "--savings"]));
    assert!(out.contains("saved: nothing"), "{out}");
}

#[test]
fn savings_on_an_empty_selection_still_answer_in_json() {
    let root = sandbox("savings-empty");
    let o = tokenburn(&root, &["--tool", "amp", "--all", "--json", "-s"]);
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
    assert_eq!(v["report"].as_array().unwrap().len(), 0);
    assert_eq!(v["savings"]["tokens"], 0);
}

#[test]
fn a_bad_input_price_is_rejected() {
    let root = sandbox("savings-bad-price");
    let o = tokenburn(&root, &["--input-price", "-3"]);
    assert!(!o.status.success());
}

//! `run_limit_check` against a sandbox (it reads the process environment, so this
//! file holds a single test).

use tokenburn_core::{run_limit_check, Level, Period, Settings};

#[test]
fn the_check_reads_saved_limits_and_only_the_live_one_logs_alerts() {
    let root = std::env::temp_dir().join(format!("tokenburn-core-limits-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let proj = root.join("sessions/--p--");
    std::fs::create_dir_all(&proj).unwrap();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    std::fs::write(
        proj.join("a.jsonl"),
        format!(
            r#"{{"timestamp":"{now}","message":{{"usage":{{"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"cost":{{"total":0.9}}}}}}}}"#
        ),
    )
    .unwrap();
    std::env::set_var("TOKENBURN_PI_SESSIONS", root.join("sessions"));
    std::env::set_var("TOKENBURN_ZED_DB", root.join("none.db"));
    std::env::set_var("TOKENBURN_RTK_DB", root.join("rtk.db"));
    std::env::set_var("TOKENBURN_CONFIG_DIR", root.join("config"));

    // Nothing saved: nothing to check.
    let c = run_limit_check(false, |v| format!("${v}"));
    assert!(c.limits.is_empty() && c.statuses.is_empty() && c.alerts.is_empty());

    let mut s = Settings::default();
    s.limits.set(Period::Day, Some(1.0)); // 90 %: a warning
    s.limits.notify = false; // never send a real notification from a test
    tokenburn_core::settings::save(&s).unwrap();

    // A read-only check reports the state but raises and logs nothing.
    let c = run_limit_check(false, |v| format!("${v}"));
    assert_eq!(c.statuses.len(), 1);
    assert_eq!(
        (c.statuses[0].level, c.statuses[0].percent()),
        (Level::Warn, 90)
    );
    assert!(c.alerts.is_empty());
    assert!(!root.join("config/alerts.json").exists());

    // The live one raises it once and remembers.
    let first = run_limit_check(true, |v| format!("${v}"));
    assert_eq!(first.alerts.len(), 1);
    assert!(root.join("config/alerts.json").exists());
    assert!(run_limit_check(true, |v| format!("${v}")).alerts.is_empty());
}

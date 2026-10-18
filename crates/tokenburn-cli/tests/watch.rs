//! One watcher pass, in-process (it reads the process environment, so this file
//! holds a single test).

use tokenburn_cli::limits::{alert_line, watch_once};

#[test]
fn a_watch_pass_alerts_once_per_level() {
    let root = std::env::temp_dir().join(format!("tokenburn-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let proj = root.join("sessions/--demo--");
    std::fs::create_dir_all(&proj).unwrap();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    std::fs::write(
        proj.join("a.jsonl"),
        format!(
            r#"{{"timestamp":"{now}","message":{{"usage":{{"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"cost":{{"total":0.25}}}}}}}}"#
        ),
    )
    .unwrap();
    std::env::set_var("TOKENBURN_PI_SESSIONS", root.join("sessions"));
    std::env::set_var("TOKENBURN_ZED_DB", root.join("none.db"));
    std::env::set_var("TOKENBURN_RTK_DB", root.join("rtk.db"));
    std::env::set_var("TOKENBURN_CONFIG_DIR", root.join("config"));

    let eur = tokenburn_core::currency_by_code("USD").unwrap();
    let mut out = Vec::new();
    // No limits: nothing to say.
    assert!(watch_once(&eur, false, &mut out).unwrap().is_empty());

    let mut s = tokenburn_core::Settings::default();
    s.limits.set(tokenburn_core::Period::Day, Some(0.2));
    tokenburn_core::settings::save(&s).unwrap();

    let alerts = watch_once(&eur, false, &mut out).unwrap();
    assert_eq!(alerts.len(), 1);
    let text = String::from_utf8(out.clone()).unwrap();
    assert!(
        text.contains("daily limit exceeded") && text.contains("$0.25 of $0.20"),
        "{text}"
    );
    assert_eq!(alert_line(&alerts[0], &eur), text.trim());
    // The same state raises nothing the second time.
    assert!(watch_once(&eur, false, &mut out).unwrap().is_empty());
}

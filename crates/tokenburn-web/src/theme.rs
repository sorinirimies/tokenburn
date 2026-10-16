//! Colour themes for the dashboard: the core's palettes as CSS custom properties.
//!
//! `/style.css` is written against `var(--bg)`, `var(--accent)` … with the Default
//! theme as fallback; a `<style>` block from [`css`] (re-emitted by the live shard
//! whenever the theme select changes) overrides those variables.

use tokenburn_core::{
    Rgb, SYSTEM_THEME_NAME, THEME_COUNT, THEME_NAMES, ThemeChoice, Tool, settings, theme_by_index,
};

/// Pseudo-index meaning "follow the system" (one past the last preset).
pub const SYSTEM_INDEX: usize = THEME_COUNT;

/// CSS custom properties for the theme at `index` (clamped), as one `:root{…}` rule.
///
/// [`SYSTEM_INDEX`] yields the dark preset plus a `prefers-color-scheme: light`
/// override, so the browser follows the OS live (no server polling needed).
pub fn css(index: usize) -> String {
    if index == SYSTEM_INDEX {
        let dark = css_for(tokenburn_core::system_theme_index(true));
        let light = css_for(tokenburn_core::system_theme_index(false));
        return format!("{dark}@media (prefers-color-scheme:light){{{light}}}");
    }
    css_for(index.min(THEME_COUNT - 1))
}

fn css_for(index: usize) -> String {
    let t = theme_by_index(index);
    let mut vars: Vec<(String, Rgb)> = vec![
        ("bg".into(), t.background),
        ("panel".into(), t.surface),
        ("line".into(), t.border),
        ("sel".into(), t.selection),
        ("fg".into(), t.text_primary),
        ("fg2".into(), t.text_secondary),
        ("dim".into(), t.text_muted),
        ("accent".into(), t.accent),
        ("ok".into(), t.success),
        ("warn".into(), t.warning),
        ("err".into(), t.error),
    ];
    for (i, c) in t.series.iter().enumerate() {
        vars.push((format!("s{i}"), *c));
    }
    let body: String = vars
        .iter()
        .map(|(k, c)| format!("--{k}:{};", c.hex()))
        .collect();
    // The extra rules hold quotes and `>` selectors, which HTML escaping would corrupt
    // inside an inline `<style>`, so they are served at `/cyberpunk.css` and imported.
    let import = if THEME_NAMES[index] == "Cyberpunk" {
        "@import url(/cyberpunk.css);"
    } else {
        ""
    };
    format!(
        "{import}:root{{color-scheme:{};{body}}}",
        if t.is_dark { "dark" } else { "light" }
    )
}

/// Extra component styling for the Cyberpunk theme (cyberpunk.net look): yellow
/// slab buttons with clipped corners, uppercase display type, hard-edged panels
/// with a cut corner, neon glow and a faint scanline overlay.
pub const CYBERPUNK_CSS: &str = r#"
body{font-family:"Rajdhani","Oxanium","Bahnschrift","Arial Narrow",system-ui,sans-serif;letter-spacing:.02em;background:var(--bg) repeating-linear-gradient(0deg,transparent 0 2px,rgba(252,238,10,.025) 2px 3px);}
h1{text-transform:uppercase;letter-spacing:.14em;font-weight:700;text-shadow:2px 0 var(--err),-2px 0 var(--s4);}
h2{color:var(--s4);letter-spacing:.18em;font-weight:700;}
h2::before{content:"// ";color:var(--accent);}
.card,.panel,.empty,.warn,table,.table-wrap,.selbar .chip,.badge,.tabs .tab,select,button,.selbar .clear,.bar,.bar>span{border-radius:0;}
.card,.panel,.empty,.warn{border:1px solid var(--line);border-left:3px solid var(--accent);clip-path:polygon(0 0,calc(100% - 14px) 0,100% 14px,100% 100%,14px 100%,0 calc(100% - 14px));background:linear-gradient(135deg,var(--panel),var(--bg));}
.card .value{text-shadow:0 0 10px currentColor;}
button,.selbar .clear{background:var(--accent);color:#000;border:0;font-weight:700;text-transform:uppercase;letter-spacing:.12em;padding:7px 18px;cursor:pointer;clip-path:polygon(0 0,calc(100% - 10px) 0,100% 10px,100% 100%,10px 100%,0 calc(100% - 10px));transition:background .12s,box-shadow .12s,transform .12s;}
button:hover,.selbar .clear:hover{background:var(--s4);color:#000;box-shadow:0 0 14px var(--s4);}
button:active,.selbar .clear:active{transform:translate(1px,1px);background:var(--err);}
select{background:#000;color:var(--accent);border:1px solid var(--accent);text-transform:uppercase;letter-spacing:.08em;padding:6px 10px;clip-path:polygon(0 0,calc(100% - 8px) 0,100% 8px,100% 100%,8px 100%,0 calc(100% - 8px));}
select:hover,select:focus{outline:none;border-color:var(--s4);color:var(--s4);box-shadow:0 0 10px rgba(0,210,235,.5);}
.tabs .tab{background:transparent;border:1px solid var(--line);text-transform:uppercase;letter-spacing:.14em;font-weight:700;clip-path:polygon(0 0,calc(100% - 8px) 0,100% 8px,100% 100%,0 100%);}
.tabs .tab:hover{color:var(--accent);border-color:var(--accent);}
.tabs .tab.active{background:var(--accent);color:#000;border-color:var(--accent);}
table{border:1px solid var(--line);border-top:2px solid var(--accent);}
th{color:#000;background:var(--accent);text-transform:uppercase;letter-spacing:.12em;}
tr:hover td{background:var(--sel);box-shadow:inset 3px 0 var(--accent);}
tr.total td{color:var(--accent);}
.badge{text-transform:uppercase;letter-spacing:.12em;}
.badge.live{box-shadow:0 0 8px var(--ok);}
.bar{height:6px;background:#000;border:1px solid var(--line);}
.bar>span{box-shadow:0 0 8px var(--accent);}
.selbar .chip{background:transparent;border-color:var(--s4);color:var(--s4);text-transform:uppercase;letter-spacing:.1em;}
.legend .item.sel{outline-color:var(--s4);}
.legend .sw{border-radius:0;}
a{color:var(--s4);text-decoration-color:var(--accent);}
a:hover{color:var(--accent);}
::selection{background:var(--accent);color:#000;}
"#;

/// `var(--sN)` for a tool (`pi`, `claude`, …); the accent for anything else.
pub fn series_var(tool: &str) -> String {
    Tool::ALL
        .iter()
        .position(|t| t.as_str() == tool)
        .map_or_else(
            || "var(--accent)".to_string(),
            |i| format!("var(--s{})", i % 8),
        )
}

/// Pick the theme: an explicit, valid `requested` name wins, otherwise `saved`.
/// `"System"` resolves to [`SYSTEM_INDEX`].
pub fn resolve(requested: Option<&str>, saved: usize) -> usize {
    match requested.and_then(ThemeChoice::parse) {
        Some(ThemeChoice::System) => SYSTEM_INDEX,
        Some(ThemeChoice::Preset(i)) => i,
        None => saved,
    }
}

/// Theme names for the select: "System" first, then the presets in canonical order.
pub fn names() -> Vec<&'static str> {
    std::iter::once(SYSTEM_THEME_NAME)
        .chain(THEME_NAMES.iter().copied())
        .collect()
}

/// The name for a theme index ([`SYSTEM_INDEX`] → "System"; out of range is clamped).
pub fn name(index: usize) -> &'static str {
    if index == SYSTEM_INDEX {
        SYSTEM_THEME_NAME
    } else {
        THEME_NAMES[index.min(THEME_COUNT - 1)]
    }
}

/// The saved theme ([`SYSTEM_INDEX`] when following the system), shared with the TUI and GUI.
pub fn saved_index() -> usize {
    let s = settings::load();
    if s.follow_system_theme() {
        SYSTEM_INDEX
    } else {
        s.theme_index()
    }
}

/// Remember `index` for the TUI and GUI (best effort).
pub fn save_index(index: usize) {
    let _ = if index == SYSTEM_INDEX {
        settings::save_system_theme()
    } else {
        settings::save_theme(index)
    };
}

/// Every theme with its colours, for `/api/themes`.
pub fn catalogue() -> serde_json::Value {
    (0..THEME_COUNT)
        .map(|i| {
            let t = theme_by_index(i);
            serde_json::json!({
                "name": THEME_NAMES[i],
                "dark": t.is_dark,
                "background": t.background.hex(),
                "surface": t.surface.hex(),
                "border": t.border.hex(),
                "selection": t.selection.hex(),
                "text": t.text_primary.hex(),
                "text_secondary": t.text_secondary.hex(),
                "text_muted": t.text_muted.hex(),
                "accent": t.accent.hex(),
                "success": t.success.hex(),
                "warning": t.warning.hex(),
                "error": t.error.hex(),
                "series": t.series.iter().map(|c| c.hex()).collect::<Vec<_>>(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_yields_a_complete_rule() {
        for i in 0..THEME_COUNT {
            let c = css(i);
            for v in [
                "--bg:",
                "--panel:",
                "--line:",
                "--fg:",
                "--accent:",
                "--ok:",
                "--warn:",
                "--err:",
                "--s0:",
                "--s7:",
            ] {
                assert!(c.contains(v), "theme {i} lacks {v}");
            }
            assert!(c.contains(":root{") && c.ends_with('}'));
        }
    }

    #[test]
    fn css_is_safe_to_inline_in_a_style_element() {
        // The rule is interpolated as text into <style>: it must not contain anything
        // HTML would escape (which would corrupt it) or that could close the element.
        for i in 0..THEME_COUNT {
            let c = css(i);
            assert!(
                c.chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || "#:;{}-,.@()/_ ".contains(ch)),
                "theme {i}: unexpected characters in {c}"
            );
        }
    }

    #[test]
    fn dark_and_light_themes_set_the_matching_color_scheme() {
        assert!(css(0).contains("color-scheme:dark"));
        let light = (0..THEME_COUNT)
            .find(|i| !theme_by_index(*i).is_dark)
            .unwrap();
        assert!(css(light).contains("color-scheme:light"));
    }

    #[test]
    fn out_of_range_index_is_clamped() {
        assert_eq!(css(usize::MAX), css(THEME_COUNT - 1));
    }

    #[test]
    fn requested_theme_wins_only_when_valid() {
        assert_eq!(
            resolve(Some("nord"), 3),
            tokenburn_core::find_theme("Nord").unwrap()
        );
        assert_eq!(
            resolve(Some("not a theme"), 3),
            3,
            "unknown names keep the saved theme"
        );
        assert_eq!(resolve(None, 7), 7);
    }

    #[test]
    fn system_resolves_to_the_system_index_in_any_case() {
        assert_eq!(resolve(Some("System"), 7), SYSTEM_INDEX);
        assert_eq!(resolve(Some(" system "), 7), SYSTEM_INDEX);
    }

    #[test]
    fn system_css_has_a_dark_base_and_a_light_media_override() {
        let c = css(SYSTEM_INDEX);
        assert!(c.starts_with(":root{color-scheme:dark;"), "{c}");
        assert!(
            c.contains("@media (prefers-color-scheme:light){:root{color-scheme:light;"),
            "{c}"
        );
        assert!(c.ends_with("}}"), "{c}");
        let dark = css(tokenburn_core::system_theme_index(true));
        let light = css(tokenburn_core::system_theme_index(false));
        assert_eq!(
            c,
            format!("{dark}@media (prefers-color-scheme:light){{{light}}}")
        );
        assert!(
            c.chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "#:;{}-,. ()@".contains(ch)),
            "safe to inline in <style>: {c}"
        );
    }

    #[test]
    fn names_lead_with_system_and_name_inverts_the_index() {
        let n = names();
        assert_eq!(n.len(), THEME_COUNT + 1);
        assert_eq!(n[0], "System");
        assert_eq!(n[1], "Default");
        assert_eq!(name(SYSTEM_INDEX), "System");
        assert_eq!(name(0), "Default");
        assert_eq!(name(usize::MAX - 1), THEME_NAMES[THEME_COUNT - 1]);
    }

    #[test]
    fn series_vars_are_distinct_per_tool() {
        let vars: Vec<String> = Tool::ALL
            .iter()
            .take(8)
            .map(|t| series_var(t.as_str()))
            .collect();
        for (i, v) in vars.iter().enumerate() {
            assert!(!vars[..i].contains(v));
        }
        assert_eq!(series_var("mystery"), "var(--accent)");
    }

    #[test]
    fn catalogue_lists_every_theme() {
        let c = catalogue();
        assert_eq!(c.as_array().unwrap().len(), THEME_COUNT);
        assert_eq!(c[0]["name"], "Default");
        assert_eq!(c[0]["series"].as_array().unwrap().len(), 8);
    }
}

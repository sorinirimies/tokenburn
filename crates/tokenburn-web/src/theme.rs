//! Colour themes for the dashboard: the core's palettes as CSS custom properties.
//!
//! `/style.css` is written against `var(--bg)`, `var(--accent)` … with the Default
//! theme as fallback; a `<style>` block from [`css`] (re-emitted by the live shard
//! whenever the theme select changes) overrides those variables.

use tokenburn_core::{Rgb, THEME_COUNT, THEME_NAMES, Tool, find_theme, theme_by_index};

/// CSS custom properties for the theme at `index` (clamped), as one `:root{…}` rule.
pub fn css(index: usize) -> String {
    let t = theme_by_index(index.min(THEME_COUNT - 1));
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
    format!(
        ":root{{color-scheme:{};{body}}}",
        if t.is_dark { "dark" } else { "light" }
    )
}

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
pub fn resolve(requested: Option<&str>, saved: usize) -> usize {
    requested.and_then(find_theme).unwrap_or(saved)
}

/// Theme names in canonical order (for the select).
pub fn names() -> &'static [&'static str] {
    THEME_NAMES
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
            assert!(c.starts_with(":root{") && c.ends_with('}'));
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
                    .all(|ch| ch.is_ascii_alphanumeric() || "#:;{}-,. ".contains(ch)),
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
        assert_eq!(resolve(Some("nord"), 3), find_theme("Nord").unwrap());
        assert_eq!(
            resolve(Some("not a theme"), 3),
            3,
            "unknown names keep the saved theme"
        );
        assert_eq!(resolve(None, 7), 7);
    }

    #[test]
    fn series_vars_are_distinct_per_tool() {
        let vars: Vec<String> = Tool::ALL.iter().map(|t| series_var(t.as_str())).collect();
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

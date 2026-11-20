use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::types::{Price, Prices};

/// The built-in table (`prices.json`, described by `prices.schema.json`).
const BUILTIN: &str = include_str!("prices.json");

/// Spellings of one model id compare equal: case, `.` vs `-`, a `:latest` or
/// `-latest` suffix, a provider prefix (`anthropic/…`) and a trailing `-YYYYMMDD`
/// date are ignored, so `Claude-Sonnet-5.5` and `claude-sonnet-5-5-20260101` are
/// the same model.
pub fn normalize(model: &str) -> String {
    let lower = model.trim().to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    let name = name.strip_suffix(":latest").unwrap_or(name);
    let name = name.strip_suffix("-latest").unwrap_or(name);
    let name = name.replace('.', "-");
    match name.rsplit_once('-') {
        Some((head, date)) if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) => {
            head.to_string()
        }
        _ => name,
    }
}

/// `prices.json` in the TokenBurn config directory (next to `settings.json`).
pub fn prices_path() -> Option<PathBuf> {
    crate::features::settings::config_dir().map(|d| d.join("prices.json"))
}

/// One entry as written in a file; `null` removes the model.
#[derive(Debug, Deserialize)]
struct RawPrice {
    input: f64,
    output: f64,
    cache_write: Option<f64>,
    cache_read: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawFile {
    models: BTreeMap<String, Option<RawPrice>>,
}

fn sane(p: f64) -> bool {
    p.is_finite() && p >= 0.0
}

impl RawPrice {
    fn resolve(&self) -> Option<Price> {
        let write = self
            .cache_write
            .unwrap_or(self.input * Price::CACHE_WRITE_RATIO);
        let read = self
            .cache_read
            .unwrap_or(self.input * Price::CACHE_READ_RATIO);
        [self.input, self.output, write, read]
            .into_iter()
            .all(sane)
            .then_some(Price {
                input: self.input,
                output: self.output,
                cache_write: write,
                cache_read: read,
            })
    }
}

impl Prices {
    /// The built-in table only.
    pub fn builtin() -> Prices {
        let mut prices = Prices::default();
        let raw: RawFile =
            serde_json::from_str(BUILTIN).expect("the embedded prices.json is valid (tested)");
        prices.apply(raw);
        prices
    }

    /// The built-in table with the user's `prices.json` applied on top.
    ///
    /// A missing file is the built-in table; a file that cannot be parsed is
    /// ignored (and left untouched, so a hand edit can be fixed).
    pub fn load() -> Prices {
        match prices_path() {
            Some(path) => Prices::load_from(&path),
            None => Prices::builtin(),
        }
    }

    /// [`Prices::load`] with an explicit file.
    pub fn load_from(path: &Path) -> Prices {
        let mut prices = Prices::builtin();
        let Ok(text) = std::fs::read_to_string(path) else {
            return prices;
        };
        match serde_json::from_str::<RawFile>(&text) {
            Ok(raw) => prices.apply(raw),
            Err(e) => tracing::warn!("prices: ignoring {}: {e}", path.display()),
        }
        prices
    }

    /// Entries replace the same model; `null` removes it; invalid prices are skipped.
    fn apply(&mut self, raw: RawFile) {
        for (model, entry) in raw.models {
            let key = normalize(&model);
            match entry {
                None => {
                    self.models.remove(&key);
                }
                Some(p) => match p.resolve() {
                    Some(price) => {
                        self.models.insert(key, price);
                    }
                    None => tracing::warn!("prices: ignoring the invalid price of {model}"),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, text: &str) -> PathBuf {
        let p = dir.join("prices.json");
        std::fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn spellings_of_a_model_are_one_model() {
        for name in [
            "claude-sonnet-5-5",
            "claude-sonnet-5.5",
            "Claude-Sonnet-5.5",
            "anthropic/claude-sonnet-5.5",
            "claude-sonnet-5-5-20260101",
            "claude-sonnet-5.5:latest",
            "claude-sonnet-5-5-latest",
            "  claude-sonnet-5-5 ",
        ] {
            assert_eq!(normalize(name), "claude-sonnet-5-5", "{name}");
        }
        assert_eq!(normalize("gpt-5.6-terra"), "gpt-5-6-terra");
        assert_eq!(
            normalize("claude-opus-4-1-20250805"),
            "claude-opus-4-1",
            "a date is not a version"
        );
        assert_eq!(
            normalize("claude-sonnet-5"),
            "claude-sonnet-5",
            "short numbers are versions"
        );
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn the_builtin_table_parses_and_is_sane() {
        let p = Prices::builtin();
        assert!(p.len() >= 20, "{}", p.len());
        for price in p.models.values() {
            assert!(price.output >= price.input && price.input > 0.0);
            assert!(price.cache_read < price.input && price.cache_write > price.input);
        }
        // Every key is already normalized, or a lookup could never find it.
        for key in p.models.keys() {
            assert_eq!(&normalize(key), key);
        }
    }

    #[test]
    fn the_builtin_prices_are_anthropics_published_ones() {
        let p = Prices::builtin();
        let sonnet = p.get("claude-sonnet-5.5").unwrap();
        assert_eq!(
            (
                sonnet.input,
                sonnet.output,
                sonnet.cache_write,
                sonnet.cache_read
            ),
            (2.0, 10.0, 2.5, 0.10)
        );
        let opus = p.get("claude-opus-4.8").unwrap();
        assert_eq!((opus.input, opus.output), (5.0, 25.0));
        let haiku = p.get("claude-haiku-4-5").unwrap();
        assert_eq!((haiku.input, haiku.cache_read), (1.0, 0.10));
        assert!(p.get("gpt-5.6-terra").is_none(), "unknown stays unknown");
    }

    #[test]
    fn cost_adds_up_every_kind_of_token() {
        let sonnet = *Prices::builtin().get("claude-sonnet-5-5").unwrap();
        let one_million = 1_000_000;
        let c = sonnet.cost(one_million, one_million, one_million, one_million);
        assert!((c - (2.0 + 10.0 + 2.5 + 0.1)).abs() < 1e-9, "{c}");
        assert_eq!(sonnet.cost(0, 0, 0, 0), 0.0);
    }

    #[test]
    fn the_cache_prices_default_from_the_input_price() {
        let d = tempfile::tempdir().unwrap();
        let path = write(
            d.path(),
            r#"{"models":{"my-model":{"input":4,"output":8}}}"#,
        );
        let price = *Prices::load_from(&path).get("My-Model").unwrap();
        assert_eq!((price.cache_write, price.cache_read), (5.0, 0.4));
    }

    #[test]
    fn a_user_file_adds_replaces_and_removes() {
        let d = tempfile::tempdir().unwrap();
        let path = write(
            d.path(),
            r#"{"$schema":"x","models":{
                "claude-sonnet-5.5": {"input": 1, "output": 2, "cache_write": 3, "cache_read": 4},
                "claude-haiku-4-5": null,
                "local-llama": {"input": 0, "output": 0}
            }}"#,
        );
        let p = Prices::load_from(&path);
        let sonnet = p.get("claude-sonnet-5-5").unwrap();
        assert_eq!((sonnet.input, sonnet.cache_read), (1.0, 4.0), "replaced");
        assert!(p.get("claude-haiku-4.5").is_none(), "null removes");
        assert_eq!(p.get("local-llama").unwrap().input, 0.0, "free is allowed");
        assert!(p.get("claude-opus-4-8").is_some(), "the rest is kept");
    }

    #[test]
    fn a_missing_or_broken_file_is_the_builtin_table() {
        let d = tempfile::tempdir().unwrap();
        let builtin = Prices::builtin();
        assert_eq!(Prices::load_from(&d.path().join("nope.json")), builtin);
        let path = write(d.path(), "{ not json");
        assert_eq!(Prices::load_from(&path), builtin);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{ not json",
            "a hand edit is left to be fixed"
        );
        let path = write(d.path(), r#"{"models": 3}"#);
        assert_eq!(Prices::load_from(&path), builtin);
    }

    #[test]
    fn invalid_prices_are_skipped_not_applied() {
        let d = tempfile::tempdir().unwrap();
        let path = write(
            d.path(),
            r#"{"models":{
                "claude-sonnet-5-5": {"input": -1, "output": 2},
                "bad-two": {"input": 1, "output": 2, "cache_read": -3},
                "claude-opus-4-8": {"output": 2}
            }}"#,
        );
        let p = Prices::load_from(&path);
        let builtin = Prices::builtin();
        assert_eq!(p.get("claude-sonnet-5-5"), builtin.get("claude-sonnet-5-5"));
        assert!(p.get("bad-two").is_none());
        assert_eq!(
            p.get("claude-opus-4-8"),
            builtin.get("claude-opus-4-8"),
            "an entry without an input price is invalid, not zero"
        );
    }

    #[test]
    fn the_schema_describes_what_the_loader_reads() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("prices.schema.json")).unwrap();
        let entry = &schema["properties"]["models"]["additionalProperties"]["oneOf"][1];
        let mut props: Vec<&str> = entry["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        props.sort_unstable();
        assert_eq!(props, ["cache_read", "cache_write", "input", "output"]);
        assert_eq!(entry["required"], serde_json::json!(["input", "output"]));
        // The shipped table uses exactly those fields.
        let builtin: serde_json::Value = serde_json::from_str(BUILTIN).unwrap();
        for (model, p) in builtin["models"].as_object().unwrap() {
            let mut keys: Vec<&str> = p.as_object().unwrap().keys().map(String::as_str).collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                ["cache_read", "cache_write", "input", "output"],
                "{model}"
            );
        }
    }
}

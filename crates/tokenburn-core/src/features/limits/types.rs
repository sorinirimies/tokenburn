use serde::{Deserialize, Serialize};

/// How long a limit's budget lasts. Days, weeks (Monday start) and months follow
/// the local time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Day,
    Week,
    Month,
}

impl Period {
    pub const ALL: [Period; 3] = [Period::Day, Period::Week, Period::Month];

    /// `daily`, `weekly`, `monthly`.
    pub fn label(self) -> &'static str {
        match self {
            Period::Day => "daily",
            Period::Week => "weekly",
            Period::Month => "monthly",
        }
    }

    /// `today`, `this week`, `this month`.
    pub fn span(self) -> &'static str {
        match self {
            Period::Day => "today",
            Period::Week => "this week",
            Period::Month => "this month",
        }
    }
}

/// Cost limits in **US dollars** (shown converted to the display currency).
/// `None` (or a non-positive number) means no limit for that period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Limits {
    pub daily: Option<f64>,
    pub weekly: Option<f64>,
    pub monthly: Option<f64>,
    /// Fraction of a limit at which the warning starts (default 0.8).
    pub warn_at: f64,
    /// Send desktop notifications (default true). Banners show regardless.
    pub notify: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            daily: None,
            weekly: None,
            monthly: None,
            warn_at: 0.8,
            notify: true,
        }
    }
}

impl Limits {
    /// The sane limit for `period`: set, finite and positive.
    pub fn get(&self, period: Period) -> Option<f64> {
        let v = match period {
            Period::Day => self.daily,
            Period::Week => self.weekly,
            Period::Month => self.monthly,
        };
        v.filter(|v| v.is_finite() && *v > 0.0)
    }

    /// Set (or, with `None` / `0`, clear) the limit for `period`.
    pub fn set(&mut self, period: Period, usd: Option<f64>) {
        let v = usd.filter(|v| v.is_finite() && *v > 0.0);
        match period {
            Period::Day => self.daily = v,
            Period::Week => self.weekly = v,
            Period::Month => self.monthly = v,
        }
    }

    /// `true` when no period has a limit.
    pub fn is_empty(&self) -> bool {
        Period::ALL.iter().all(|p| self.get(*p).is_none())
    }

    /// The warning fraction, clamped to a usable range.
    pub fn warn_fraction(&self) -> f64 {
        if self.warn_at.is_finite() && self.warn_at > 0.0 && self.warn_at < 1.0 {
            self.warn_at
        } else {
            0.8
        }
    }
}

/// How close spending is to a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    Warn,
    Over,
}

/// One limit against what has been spent in its current period.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LimitStatus {
    pub period: Period,
    pub limit_usd: f64,
    pub spent_usd: f64,
    pub level: Level,
}

impl LimitStatus {
    /// Spent as a fraction of the limit (may exceed 1).
    pub fn ratio(&self) -> f64 {
        self.spent_usd / self.limit_usd
    }

    /// Whole percent used.
    pub fn percent(&self) -> u32 {
        (self.ratio() * 100.0).round().max(0.0) as u32
    }
}

/// A level that was just crossed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Alert {
    pub period: Period,
    pub level: Level,
    pub spent_usd: f64,
    pub limit_usd: f64,
}

impl Alert {
    /// Notification title.
    pub fn title(&self) -> String {
        match self.level {
            Level::Over => format!("TokenBurn: {} limit exceeded", self.period.label()),
            _ => format!("TokenBurn: {} limit nearly reached", self.period.label()),
        }
    }

    /// Notification body, amounts formatted by `money` (USD → display currency).
    pub fn body(&self, money: impl Fn(f64) -> String) -> String {
        format!(
            "Spent {} of {} {} ({}%)",
            money(self.spent_usd),
            money(self.limit_usd),
            self.period.span(),
            (self.spent_usd / self.limit_usd * 100.0).round() as u32
        )
    }
}

/// The limits as saved, how much of each is used, and the alerts just raised.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LimitCheck {
    pub limits: Limits,
    pub statuses: Vec<LimitStatus>,
    /// Levels crossed for the first time (not raised by any front-end before).
    pub alerts: Vec<Alert>,
}

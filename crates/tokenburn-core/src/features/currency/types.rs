use std::fmt;
use std::str::FromStr;

/// A display currency.
///
/// Every tool logs its cost in US dollars, so USD is the source of truth and a
/// currency is only a *presentation*: `per_usd` units of it per dollar. TokenBurn
/// never goes online, so the built-in rates are fixed, approximate values — set
/// exact ones under `"rates"` in `settings.json` (see [`super::resolve`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Currency {
    /// ISO 4217 code, upper case (`"EUR"`).
    pub code: &'static str,
    /// What goes in front of the number (`"€"`, or `"CHF "` where there is no
    /// common symbol).
    pub symbol: &'static str,
    pub name: &'static str,
    /// Units of this currency per US dollar.
    pub per_usd: f64,
    /// Digits after the decimal point (0 for yen and won).
    pub decimals: usize,
    /// The rate comes from the user's settings, not the built-in table.
    pub custom_rate: bool,
}

const fn c(
    code: &'static str,
    symbol: &'static str,
    name: &'static str,
    per_usd: f64,
    decimals: usize,
) -> Currency {
    Currency {
        code,
        symbol,
        name,
        per_usd,
        decimals,
        custom_rate: false,
    }
}

/// Built-in currencies and their **approximate** rates. US dollars first (it is the
/// default and the unit the logs use), then alphabetical by code.
pub const CURRENCIES: [Currency; 41] = [
    c("USD", "$", "US dollar", 1.0, 2),
    c("AED", "AED ", "UAE dirham", 3.67, 2),
    c("AUD", "A$", "Australian dollar", 1.52, 2),
    c("BGN", "лв ", "Bulgarian lev", 1.8, 2),
    c("BRL", "R$", "Brazilian real", 5.0, 2),
    c("CAD", "C$", "Canadian dollar", 1.36, 2),
    c("CHF", "CHF ", "Swiss franc", 0.88, 2),
    c("CLP", "CL$", "Chilean peso", 950.0, 0),
    c("CNY", "CN¥", "Chinese yuan", 7.2, 2),
    c("COP", "CO$", "Colombian peso", 4000.0, 0),
    c("CZK", "Kč ", "Czech koruna", 23.0, 2),
    c("DKK", "kr ", "Danish krone", 6.9, 2),
    c("EGP", "EGP ", "Egyptian pound", 48.0, 2),
    c("EUR", "€", "Euro", 0.92, 2),
    c("GBP", "£", "Pound sterling", 0.79, 2),
    c("HKD", "HK$", "Hong Kong dollar", 7.8, 2),
    c("HUF", "Ft ", "Hungarian forint", 360.0, 0),
    c("IDR", "Rp ", "Indonesian rupiah", 15500.0, 0),
    c("ILS", "₪", "Israeli shekel", 3.7, 2),
    c("INR", "₹", "Indian rupee", 83.0, 2),
    c("ISK", "kr ", "Icelandic króna", 138.0, 0),
    c("JPY", "¥", "Japanese yen", 150.0, 0),
    c("KRW", "₩", "South Korean won", 1330.0, 0),
    c("MXN", "MX$", "Mexican peso", 17.0, 2),
    c("MYR", "RM ", "Malaysian ringgit", 4.7, 2),
    c("NGN", "₦", "Nigerian naira", 1500.0, 0),
    c("NOK", "kr ", "Norwegian krone", 10.6, 2),
    c("NZD", "NZ$", "New Zealand dollar", 1.65, 2),
    c("PHP", "₱", "Philippine peso", 56.0, 2),
    c("PLN", "zł ", "Polish złoty", 4.0, 2),
    c("RON", "lei ", "Romanian leu", 4.6, 2),
    c("RUB", "₽", "Russian ruble", 90.0, 2),
    c("SAR", "SAR ", "Saudi riyal", 3.75, 2),
    c("SEK", "kr ", "Swedish krona", 10.5, 2),
    c("SGD", "S$", "Singapore dollar", 1.34, 2),
    c("THB", "฿", "Thai baht", 35.0, 2),
    c("TRY", "₺", "Turkish lira", 32.0, 2),
    c("TWD", "NT$", "New Taiwan dollar", 32.0, 2),
    c("UAH", "₴", "Ukrainian hryvnia", 40.0, 2),
    c("VND", "₫", "Vietnamese đồng", 24500.0, 0),
    c("ZAR", "R ", "South African rand", 18.5, 2),
];

impl Currency {
    pub const USD: Currency = CURRENCIES[0];

    /// US dollars → this currency.
    pub fn convert(&self, usd: f64) -> f64 {
        usd * self.per_usd
    }

    /// An amount that is **already** in this currency: `€1.38`, `¥210`, `CHF 1.20`.
    pub fn format_amount(&self, amount: f64) -> String {
        format!("{}{:.*}", self.symbol, self.decimals, amount)
    }

    /// A cost in US dollars, converted and formatted.
    pub fn format(&self, usd: f64) -> String {
        self.format_amount(self.convert(usd))
    }

    /// A chart-axis amount (already in this currency): whole units from 100 up so
    /// the labels stay short, full precision below.
    pub fn axis(&self, amount: f64) -> String {
        if amount >= 100.0 {
            format!("{}{amount:.0}", self.symbol)
        } else {
            self.format_amount(amount)
        }
    }

    /// Rates are fixed estimates unless the user set one.
    pub fn is_approximate(&self) -> bool {
        self.code != "USD" && !self.custom_rate
    }

    /// Short note for the cost label: `""` for USD or a user rate, `" ≈"` otherwise.
    pub fn approx_mark(&self) -> &'static str {
        if self.is_approximate() {
            " ≈"
        } else {
            ""
        }
    }
}

/// `"EUR · Euro"`, for pick lists.
impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} · {}", self.code, self.name)
    }
}

/// Parses a currency code, case-insensitively (`eur`, `EUR`).
impl FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        super::ops::find(s).ok_or_else(|| {
            format!(
                "unknown currency '{s}' (one of: {})",
                CURRENCIES
                    .iter()
                    .map(|c| c.code)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
}

impl Default for Currency {
    fn default() -> Self {
        Self::USD
    }
}

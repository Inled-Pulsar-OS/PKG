//! Currency rates: a pure table fed by a cache loader and a curl-based refresher.
//! Rates are stored as USD per 1 unit of the currency (USD itself is always 1).

use super::units::{Unit, CUR};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

const API: &str = "https://api.frankfurter.dev/v1/latest?base=USD";
const MAX_AGE_SECS: u64 = 12 * 3600;

#[derive(Clone, Debug, Default)]
pub struct CurrencyTable {
    /// code (lowercase) -> USD per unit
    usd_per: HashMap<String, f64>,
    pub date: String,
    pub fetched: u64,
}

impl CurrencyTable {
    /// Build from "units of X per 1 USD" rates (as frankfurter returns with base=USD).
    pub fn from_usd_base(rates: &HashMap<String, f64>, date: &str, fetched: u64) -> Self {
        let mut usd_per = HashMap::new();
        for (k, v) in rates {
            if *v > 0.0 {
                usd_per.insert(k.to_lowercase(), 1.0 / v);
            }
        }
        usd_per.insert("usd".into(), 1.0);
        Self { usd_per, date: date.into(), fetched }
    }

    pub fn has(&self, code: &str) -> bool {
        code == "usd" || self.usd_per.contains_key(code)
    }

    pub fn len(&self) -> usize {
        self.usd_per.len().max(1)
    }

    pub fn unit(&self, name: &str) -> Option<Unit> {
        let code = match name {
            "dollar" | "dollars" | "buck" | "bucks" | "$" => "usd",
            "euro" | "euros" | "€" => "eur",
            "yen" | "¥" => "jpy",
            "£" | "sterling" => "gbp",
            n => n,
        };
        let f = if code == "usd" { 1.0 } else { *self.usd_per.get(code)? };
        let mut dim = [0i8; 8];
        dim[CUR] = 1;
        Some(Unit {
            sym: code.to_uppercase(),
            long: code.to_uppercase(),
            dim,
            f,
            off: 0.0,
            expand: None,
        })
    }

    fn parse(json: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(json).ok()?;
        let base = v.get("base").and_then(|b| b.as_str()).unwrap_or("USD").to_uppercase();
        let mut rates: HashMap<String, f64> = v
            .get("rates")?
            .as_object()?
            .iter()
            .filter_map(|(k, x)| Some((k.clone(), x.as_f64()?)))
            .collect();
        if base != "USD" {
            // Re-base: rates are per 1 base; convert to per 1 USD.
            let usd = *rates.get("USD")?;
            rates.insert(base, 1.0);
            for r in rates.values_mut() {
                *r /= usd;
            }
        }
        let date = v.get("date").and_then(|d| d.as_str()).unwrap_or("");
        let fetched = v.get("fetched").and_then(|d| d.as_u64()).unwrap_or(0);
        Some(Self::from_usd_base(&rates, date, fetched))
    }

    pub fn load_cache(path: &std::path::Path) -> Option<Self> {
        Self::parse(&std::fs::read_to_string(path).ok()?)
    }
}

pub fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("pulsaros-spotlight")
        .join("currencies.json")
}

/// `currency_rates` in ~/.config/pulsaros-spotlight/config.json (default true).
pub fn rates_enabled() -> bool {
    dirs::config_dir()
        .map(|d| d.join("pulsaros-spotlight/config.json"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("currency_rates").and_then(|b| b.as_bool()))
        .unwrap_or(true)
}

fn global() -> &'static RwLock<CurrencyTable> {
    static G: OnceLock<RwLock<CurrencyTable>> = OnceLock::new();
    G.get_or_init(|| {
        RwLock::new(CurrencyTable::load_cache(&cache_path()).unwrap_or_default())
    })
}

pub fn current() -> CurrencyTable {
    global().read().map(|g| g.clone()).unwrap_or_default()
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Blocking: fetch ECB rates via curl when the cache is stale. Call from a worker thread.
/// Returns true when a fresh table was stored.
pub fn refresh_currencies() -> bool {
    if !rates_enabled() {
        return false;
    }
    let path = cache_path();
    if let Some(t) = CurrencyTable::load_cache(&path) {
        if t.fetched > 0 && now_secs().saturating_sub(t.fetched) < MAX_AGE_SECS {
            if let Ok(mut g) = global().write() {
                *g = t;
            }
            return false;
        }
    }
    let out = Command::new("curl").args(["-fsS", "--max-time", "10", API]).output();
    let Ok(out) = out else { return false };
    if !out.status.success() {
        return false;
    }
    let Ok(body) = String::from_utf8(out.stdout) else { return false };
    let Some(mut table) = CurrencyTable::parse(&body) else { return false };
    table.fetched = now_secs();
    if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&body) {
        v["fetched"] = serde_json::json!(table.fetched);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, v.to_string()).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
    if let Ok(mut g) = global().write() {
        *g = table;
    }
    true
}

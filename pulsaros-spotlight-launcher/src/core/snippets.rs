//! Snippets (V3): model, storage (`db/snippets.json`), search and the placeholder engine.
//!
//! `render(text, &Env) -> Rendered`. Placeholders: `{clipboard}`, `{date}` (`%Y-%m-%d`),
//! `{date:<strftime>}`, `{time}` (`%H:%M:%S`), `{datetime}`, `{uuid}`, `{cursor}`,
//! `{argument name="x" default="y"}`. Unknown placeholders (and unterminated braces) stay literal.
//! `cursor_offset` is a char (not byte) offset into `Rendered::text` of the first `{cursor}`; further
//! `{cursor}`s render empty. Arguments are collected (first occurrence wins) in `Rendered::arguments`;
//! values come from `Env::args`, then the default, then empty. Date specifiers: `%Y %y %m %d %e %H %I
//! %M %S %p %j %a %A %b %B %F %T %z %%`; unknown specifiers are kept literally.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ranker::{self, Item};
use super::store::Store;

const FILE: &str = "snippets";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub keyword: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snippets {
    #[serde(default)]
    pub items: Vec<Snippet>,
}

impl Snippets {
    pub fn load(store: &Store) -> Self {
        store.load(FILE)
    }

    pub fn save(&self, store: &Store) -> std::io::Result<()> {
        store.save(FILE, self)
    }

    /// Add a snippet and return its new id. Rejects empty name/text and a keyword used elsewhere.
    pub fn add(&mut self, name: &str, keyword: &str, text: &str) -> Result<String, String> {
        let (name, keyword) = (name.trim(), keyword.trim());
        self.check(name, keyword, text)?;
        let n = self.items.iter().filter_map(|s| s.id.strip_prefix('s')?.parse::<u64>().ok()).max().unwrap_or(0);
        let id = format!("s{}", n + 1);
        self.items.push(Snippet { id: id.clone(), name: name.into(), keyword: keyword.into(), text: text.into() });
        Ok(id)
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let n = self.items.len();
        self.items.retain(|s| s.id != id);
        self.items.len() != n
    }

    fn check(&self, name: &str, keyword: &str, text: &str) -> Result<(), String> {
        if name.is_empty() || text.is_empty() {
            return Err("name and text are required".into());
        }
        let dup = !keyword.is_empty()
            && self.items.iter().any(|s| s.keyword.eq_ignore_ascii_case(keyword));
        if dup {
            return Err(format!("keyword \"{keyword}\" is already used"));
        }
        Ok(())
    }

    /// Empty query: all by name. Otherwise ranked; the keyword acts as an alias (exact keyword wins).
    pub fn search(&self, query: &str) -> Vec<&Snippet> {
        if query.trim().is_empty() {
            let mut v: Vec<&Snippet> = self.items.iter().collect();
            v.sort_by_key(|s| s.name.to_lowercase());
            return v;
        }
        let items: Vec<Item> = self
            .items
            .iter()
            .map(|s| Item {
                id: s.id.clone(),
                title: s.name.clone(),
                keywords: vec![],
                alias: (!s.keyword.is_empty()).then(|| s.keyword.clone()),
            })
            .collect();
        ranker::rank(query, &items)
            .into_iter()
            .filter_map(|(it, _)| self.items.iter().find(|s| s.id == it.id))
            .collect()
    }
}

/// Everything the engine needs from the outside world (injected for tests).
pub struct Env {
    pub clipboard: String,
    /// Unix seconds.
    pub now: i64,
    /// Local offset from UTC in seconds.
    pub utc_offset: i64,
    pub uuid: Box<dyn Fn() -> String>,
    /// Values for `{argument}` placeholders by name.
    pub args: HashMap<String, String>,
}

impl Env {
    /// Real clock (UTC offset from `utc_offset`, caller supplies it) and random v4 uuids.
    pub fn system(clipboard: String, utc_offset: i64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        Env { clipboard, now, utc_offset, uuid: Box::new(uuid_v4), args: HashMap::new() }
    }
}

fn uuid_v4() -> String {
    use std::io::Read;
    let mut b = [0u8; 16];
    let ok = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_ok();
    if !ok {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        b = n.to_le_bytes();
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArgSpec {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rendered {
    pub text: String,
    pub cursor_offset: Option<usize>,
    pub arguments: Vec<ArgSpec>,
}

pub fn render(text: &str, env: &Env) -> Rendered {
    let mut out = Rendered::default();
    let mut buf = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('{') {
        buf.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match find_close(after) {
            Some(end) => match expand(&after[..end], env, &mut out, &buf) {
                Some(s) => {
                    buf.push_str(&s);
                    rest = &after[end + 1..];
                }
                None => {
                    buf.push('{');
                    rest = after;
                }
            },
            None => {
                buf.push('{');
                rest = after;
            }
        }
    }
    buf.push_str(rest);
    out.text = buf;
    out
}

/// Index of the `}` closing a placeholder, honoring quotes; `None` if a nested `{` comes first.
fn find_close(s: &str) -> Option<usize> {
    let mut quote = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => quote = !quote,
            '}' if !quote => return Some(i),
            '{' if !quote => return None,
            _ => {}
        }
    }
    None
}

/// `None` = unknown placeholder, keep literally. `so_far` is the output before this placeholder.
fn expand(inner: &str, env: &Env, out: &mut Rendered, so_far: &str) -> Option<String> {
    let (name, arg) = match inner.find(|c: char| c == ':' || c.is_whitespace()) {
        Some(i) => (&inner[..i], Some(&inner[i..])),
        None => (inner, None),
    };
    let fmt = |f: &str| format_time(f, env.now, env.utc_offset);
    match (name, arg) {
        ("clipboard", None) => Some(env.clipboard.clone()),
        ("uuid", None) => Some((env.uuid)()),
        ("time", None) => Some(fmt("%H:%M:%S")),
        ("date", None) => Some(fmt("%Y-%m-%d")),
        ("datetime", None) => Some(fmt("%Y-%m-%d %H:%M:%S")),
        ("date", Some(a)) => a.strip_prefix(':').filter(|f| !f.is_empty()).map(fmt),
        ("cursor", None) => {
            if out.cursor_offset.is_none() {
                out.cursor_offset = Some(so_far.chars().count());
            }
            Some(String::new())
        }
        ("argument", Some(a)) => {
            let attrs = parse_attrs(a)?;
            let get = |k: &str| attrs.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
            let name = get("name").filter(|n| !n.is_empty())?;
            if !out.arguments.iter().any(|x| x.name == name) {
                out.arguments.push(ArgSpec { name: name.clone(), default: get("default") });
            }
            let default = get("default").or_else(|| out.arguments.iter().find(|x| x.name == name)?.default.clone());
            Some(env.args.get(&name).cloned().or(default).unwrap_or_default())
        }
        _ => None,
    }
}

/// `key="value" key2="value2"`; `None` on malformed input.
fn parse_attrs(s: &str) -> Option<Vec<(String, String)>> {
    let mut v = Vec::new();
    let mut r = s.trim_start();
    while !r.is_empty() {
        let eq = r.find('=')?;
        let key = r[..eq].trim();
        let val = r[eq + 1..].strip_prefix('"')?;
        let close = val.find('"')?;
        if key.is_empty() || key.contains(char::is_whitespace) {
            return None;
        }
        v.push((key.to_string(), val[..close].to_string()));
        r = val[close + 1..].trim_start();
    }
    Some(v)
}

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November",
    "December",
];
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// Days since 1970-01-01 -> (year, month 1-12, day 1-31). Hinnant's civil_from_days.
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

pub fn format_time(fmt: &str, now: i64, utc_offset: i64) -> String {
    let local = now + utc_offset;
    let days = local.div_euclid(86_400);
    let sod = local.rem_euclid(86_400);
    let (y, mo, d) = civil(days);
    let (h, mi, s) = (sod / 3600, sod % 3600 / 60, sod % 60);
    let wd = (days + 4).rem_euclid(7) as usize;
    let yday = days - (days_from_civil(y, 1, 1)) + 1;
    let mut out = String::new();
    let mut it = fmt.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('Y') => out += &y.to_string(),
            Some('y') => out += &format!("{:02}", y.rem_euclid(100)),
            Some('m') => out += &format!("{mo:02}"),
            Some('d') => out += &format!("{d:02}"),
            Some('e') => out += &format!("{d:2}"),
            Some('H') => out += &format!("{h:02}"),
            Some('I') => out += &format!("{:02}", if h % 12 == 0 { 12 } else { h % 12 }),
            Some('M') => out += &format!("{mi:02}"),
            Some('S') => out += &format!("{s:02}"),
            Some('p') => out += if h < 12 { "AM" } else { "PM" },
            Some('j') => out += &format!("{yday:03}"),
            Some('a') => out += &DAYS[wd][..3],
            Some('A') => out += DAYS[wd],
            Some('b') => out += &MONTHS[mo as usize - 1][..3],
            Some('B') => out += MONTHS[mo as usize - 1],
            Some('F') => out += &format!("{y}-{mo:02}-{d:02}"),
            Some('T') => out += &format!("{h:02}:{mi:02}:{s:02}"),
            Some('z') => {
                let a = utc_offset.abs();
                out += &format!("{}{:02}{:02}", if utc_offset < 0 { '-' } else { '+' }, a / 3600, a % 3600 / 60);
            }
            Some('%') => out.push('%'),
            Some(o) => {
                out.push('%');
                out.push(o);
            }
            None => out.push('%'),
        }
    }
    out
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

#[cfg(test)]
#[path = "../tests/snippets_tests.rs"]
mod tests;

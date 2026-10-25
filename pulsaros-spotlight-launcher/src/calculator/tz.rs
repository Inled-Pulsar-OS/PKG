//! Time zones from a fixed table with rule-based DST. Std only.

use super::dates::{civil_from_days, days_from_civil, fmt_clock, fmt_date, month_len, weekday};
use super::{CalcResult, Ctx};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
enum Rule {
    None,
    Us,
    Eu,
    Au,
    Nz,
}

struct Zone {
    names: &'static str, // comma separated, lowercase
    city: &'static str,
    std: i32, // minutes east of UTC
    rule: Rule,
    abbr: (&'static str, &'static str),
}

macro_rules! z {
    ($n:expr, $c:expr, $o:expr, $r:ident, $a:expr, $b:expr) => {
        Zone { names: $n, city: $c, std: $o, rule: Rule::$r, abbr: ($a, $b) }
    };
}

const ZONES: &[Zone] = &[
    z!("utc,gmt,z,zulu", "UTC", 0, None, "UTC", "UTC"),
    z!("london,uk,england,bst,britain", "London", 0, Eu, "GMT", "BST"),
    z!("dublin,ireland", "Dublin", 0, Eu, "GMT", "IST"),
    z!("lisbon,portugal", "Lisbon", 0, Eu, "WET", "WEST"),
    z!("reykjavik,iceland", "Reykjavik", 0, None, "GMT", "GMT"),
    z!("paris,france,cet,cest,berlin,germany,madrid,spain,rome,italy,amsterdam,netherlands,brussels,belgium,vienna,austria,zurich,switzerland,stockholm,sweden,oslo,norway,copenhagen,denmark,prague,warsaw,poland,budapest,europe", "Paris", 60, Eu, "CET", "CEST"),
    z!("athens,greece,helsinki,finland,kyiv,kiev,ukraine,bucharest,romania,sofia,eet,eest", "Athens", 120, Eu, "EET", "EEST"),
    z!("istanbul,turkey,moscow,russia,riyadh,saudi arabia,nairobi,kenya,baghdad,msk", "Moscow", 180, None, "MSK", "MSK"),
    z!("cairo,egypt,johannesburg,south africa,cape town,harare,maputo,cat", "Johannesburg", 120, None, "SAST", "SAST"),
    z!("lagos,nigeria,kinshasa,wat", "Lagos", 60, None, "WAT", "WAT"),
    z!("dubai,uae,abu dhabi,muscat,gst", "Dubai", 240, None, "GST", "GST"),
    z!("tehran,iran", "Tehran", 210, None, "IRST", "IRST"),
    z!("karachi,pakistan,tashkent", "Karachi", 300, None, "PKT", "PKT"),
    z!("delhi,new delhi,mumbai,kolkata,india,bangalore,chennai,ist", "Delhi", 330, None, "IST", "IST"),
    z!("kathmandu,nepal", "Kathmandu", 345, None, "NPT", "NPT"),
    z!("dhaka,bangladesh", "Dhaka", 360, None, "BST", "BST"),
    z!("bangkok,thailand,jakarta,indonesia,hanoi,vietnam,ho chi minh", "Bangkok", 420, None, "ICT", "ICT"),
    z!("singapore,hong kong,hk,beijing,china,shanghai,taipei,taiwan,manila,philippines,kuala lumpur,malaysia,perth,awst,cst china", "Singapore", 480, None, "SGT", "SGT"),
    z!("tokyo,japan,jst,osaka,kyoto", "Tokyo", 540, None, "JST", "JST"),
    z!("seoul,korea,south korea,kst", "Seoul", 540, None, "KST", "KST"),
    z!("adelaide,acst,acdt", "Adelaide", 570, Au, "ACST", "ACDT"),
    z!("darwin", "Darwin", 570, None, "ACST", "ACST"),
    z!("sydney,melbourne,canberra,australia,aest,aedt", "Sydney", 600, Au, "AEST", "AEDT"),
    z!("brisbane,queensland", "Brisbane", 600, None, "AEST", "AEST"),
    z!("auckland,wellington,new zealand,nz,nzst,nzdt", "Auckland", 720, Nz, "NZST", "NZDT"),
    z!("honolulu,hawaii,hst", "Honolulu", -600, None, "HST", "HST"),
    z!("anchorage,alaska,akst,akdt", "Anchorage", -540, Us, "AKST", "AKDT"),
    z!("los angeles,la,san francisco,sf,seattle,portland,san diego,las vegas,vancouver,california,pacific,pst,pdt,pt", "Los Angeles", -480, Us, "PST", "PDT"),
    z!("denver,salt lake city,calgary,edmonton,mountain,mst,mdt,mt", "Denver", -420, Us, "MST", "MDT"),
    z!("phoenix,arizona", "Phoenix", -420, None, "MST", "MST"),
    z!("chicago,dallas,houston,austin,minneapolis,winnipeg,new orleans,texas,central,cst,cdt,ct", "Chicago", -360, Us, "CST", "CDT"),
    z!("mexico city,mexico,guatemala,costa rica", "Mexico City", -360, None, "CST", "CST"),
    z!("new york,nyc,ny,boston,miami,atlanta,toronto,washington,washington dc,dc,philadelphia,detroit,montreal,eastern,est,edt,et", "New York", -300, Us, "EST", "EDT"),
    z!("bogota,colombia,lima,peru,quito", "Bogota", -300, None, "COT", "COT"),
    z!("halifax,atlantic,ast,adt", "Halifax", -240, Us, "AST", "ADT"),
    z!("caracas,venezuela,la paz,santiago,chile", "Caracas", -240, None, "VET", "VET"),
    z!("sao paulo,rio,brazil,buenos aires,argentina,montevideo", "Sao Paulo", -180, None, "BRT", "BRT"),
    z!("st johns,newfoundland", "St. John's", -210, Us, "NST", "NDT"),
];

lazy_static::lazy_static! {
    static ref INDEX: HashMap<&'static str, usize> = {
        let mut m = HashMap::new();
        for (i, z) in ZONES.iter().enumerate() {
            for n in z.names.split(',') {
                m.entry(n).or_insert(i);
            }
        }
        m
    };
}

fn last_sunday(y: i64, m: i64) -> i64 {
    let last = days_from_civil(y, m, month_len(y, m));
    last - ((weekday(last) as i64 + 1) % 7)
}

fn first_sunday(y: i64, m: i64) -> i64 {
    let f = days_from_civil(y, m, 1);
    f + (6 - weekday(f) as i64)
}

fn is_dst(z: &Zone, ts: i64) -> bool {
    let y = civil_from_days(ts.div_euclid(86400)).0;
    let std = z.std as i64 * 60;
    match z.rule {
        Rule::None => false,
        Rule::Us => {
            let start = (first_sunday(y, 3) + 7) * 86400 + 7200 - std;
            let end = first_sunday(y, 11) * 86400 + 7200 - (std + 3600);
            ts >= start && ts < end
        }
        Rule::Eu => {
            let start = last_sunday(y, 3) * 86400 + 3600;
            let end = last_sunday(y, 10) * 86400 + 3600;
            ts >= start && ts < end
        }
        Rule::Au => {
            let start = first_sunday(y, 10) * 86400 + 7200 - std;
            let end = first_sunday(y, 4) * 86400 + 10800 - (std + 3600);
            ts >= start || ts < end
        }
        Rule::Nz => {
            let start = last_sunday(y, 9) * 86400 + 7200 - std;
            let end = first_sunday(y, 4) * 86400 + 10800 - (std + 3600);
            ts >= start || ts < end
        }
    }
}

#[derive(Clone, Copy)]
pub enum Sel {
    Zone(usize),
    Fixed(i32),
    Local,
}

impl Sel {
    /// (offset minutes, label)
    fn at(&self, ctx: &Ctx, ts: i64) -> (i32, String) {
        match *self {
            Sel::Zone(i) => {
                let z = &ZONES[i];
                let dst = is_dst(z, ts);
                (z.std + if dst { 60 } else { 0 }, format!("{} ({})", z.city, if dst { z.abbr.1 } else { z.abbr.0 }))
            }
            Sel::Fixed(m) => (m, fmt_utc_off(m)),
            Sel::Local => (ctx.local_minutes(ts), "Local time".to_string()),
        }
    }
}

fn fmt_utc_off(m: i32) -> String {
    let (s, a) = if m < 0 { ('-', -m) } else { ('+', m) };
    if a % 60 == 0 { format!("UTC{s}{}", a / 60) } else { format!("UTC{s}{}:{:02}", a / 60, a % 60) }
}

pub fn find_zone(s: &str) -> Option<Sel> {
    let s = s.trim().trim_start_matches("the ").trim_end_matches(" time");
    if matches!(s, "local" | "here" | "my time" | "local time" | "me") {
        return Some(Sel::Local);
    }
    if let Some(&i) = INDEX.get(s) {
        return Some(Sel::Zone(i));
    }
    let re = re!(r"^(?:utc|gmt)([+-])(\d{1,2})(?::?(\d{2}))?$");
    let c = re.captures(s)?;
    let m = c[2].parse::<i32>().ok()? * 60 + c.get(3).and_then(|x| x.as_str().parse::<i32>().ok()).unwrap_or(0);
    Some(Sel::Fixed(if &c[1] == "-" { -m } else { m }))
}

fn parse_clock(s: &str) -> Option<i64> {
    match s {
        "noon" => return Some(12 * 3600),
        "midnight" => return Some(0),
        _ => {}
    }
    let re = re!(r"^(\d{1,2})(?::(\d{2}))?\s?(am|pm)?$");
    let c = re.captures(s)?;
    let mut h: i64 = c[1].parse().ok()?;
    let m: i64 = c.get(2).map_or(0, |x| x.as_str().parse().unwrap_or(0));
    match c.get(3).map(|x| x.as_str()) {
        Some(ap) => {
            if h < 1 || h > 12 {
                return None;
            }
            h %= 12;
            if ap == "pm" { h += 12 }
        }
        None if c.get(2).is_none() => return None, // bare number is not a time
        None => {}
    }
    (h < 24 && m < 60).then_some(h * 3600 + m * 60)
}

fn res(q: &str, answer: String, cq: String, ca: String) -> Option<CalcResult> {
    Some(CalcResult { question: q.to_string(), answer, caption_question: cq, caption_answer: ca })
}

pub fn query(ctx: &Ctx, q: &str) -> Option<CalcResult> {
    // current time in a zone
    let now_in = re!(r"^(?:what(?:'s| is) the time|what time is it|current time|time|now|date|clock)(?: right now)? (?:in|at) (.+)$|^time (.+)$|^(.+) (?:time|clock)$");
    if let Some(c) = now_in.captures(q) {
        let z = c.get(1).or(c.get(2)).or(c.get(3))?.as_str();
        if let Some(sel) = find_zone(z) {
            if matches!(sel, Sel::Local) && c.get(3).is_none() && z != "local" && z != "here" {
                return None;
            }
            let (off, label) = sel.at(ctx, ctx.now);
            let t = ctx.now + off as i64 * 60;
            let (loff, _) = Sel::Local.at(ctx, ctx.now);
            let ldiff = (off - loff) as f64 / 60.0;
            return res(
                q,
                fmt_clock(t),
                label,
                format!("{}{}", fmt_date(t.div_euclid(86400)), if ldiff == 0.0 { String::new() } else { format!(" · {:+}h from local", ldiff) }),
            );
        }
        return None;
    }

    // conversion: "3pm london to nyc", "15:00 utc in tokyo", "3pm to tokyo"
    let conv = re!(r"^(\d{1,2}(?::\d{2})?\s?(?:am|pm)?|noon|midnight)(?: (?:in|at))?(?: (.+?))? (?:to|in|into|->) (.+)$");
    let c = conv.captures(q)?;
    let secs = parse_clock(&c[1])?;
    let src = match c.get(2) {
        Some(s) => find_zone(s.as_str())?,
        None => Sel::Local,
    };
    let dst = find_zone(&c[3])?;
    // The clock reading is on today's date in the source zone.
    let (soff0, _) = src.at(ctx, ctx.now);
    let day = (ctx.now + soff0 as i64 * 60).div_euclid(86400);
    let mut ts = day * 86400 + secs - soff0 as i64 * 60;
    let (soff, slabel) = src.at(ctx, ts);
    ts += (soff0 - soff) as i64 * 60;
    let (doff, dlabel) = dst.at(ctx, ts);
    let t = ts + doff as i64 * 60;
    let shift = t.div_euclid(86400) - day;
    let note = match shift {
        0 => String::new(),
        1 => " (next day)".into(),
        -1 => " (previous day)".into(),
        n => format!(" ({n:+} days)"),
    };
    res(
        q,
        fmt_clock(t),
        format!("{} · {}", slabel, fmt_clock(secs)),
        format!("{}{}", dlabel, note),
    )
}

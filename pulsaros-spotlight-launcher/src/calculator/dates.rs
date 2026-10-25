//! Civil-calendar helpers, date phrases, work days and Unix time. Std only.

use super::{CalcResult, Ctx};

pub const MONTHS: [&str; 12] = [
    "january", "february", "march", "april", "may", "june", "july", "august", "september",
    "october", "november", "december",
];
pub const WEEKDAYS: [&str; 7] =
    ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];

/// Days since 1970-01-01 for a proleptic Gregorian date.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 0 = Monday .. 6 = Sunday.
pub fn weekday(days: i64) -> usize {
    (days + 3).rem_euclid(7) as usize
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn month_len(y: i64, m: i64) -> i64 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

pub fn add_months(days: i64, n: i64) -> i64 {
    let (y, m, d) = civil_from_days(days);
    let t = y * 12 + (m - 1) + n;
    let (ny, nm) = (t.div_euclid(12), t.rem_euclid(12) + 1);
    days_from_civil(ny, nm, d.min(month_len(ny, nm)))
}

pub fn fmt_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{}, {} {} {}", &cap(WEEKDAYS[weekday(days)])[..3], &cap(MONTHS[m as usize - 1])[..3], d, y)
}

pub fn fmt_hms(secs: i64) -> String {
    let s = secs.rem_euclid(86400);
    format!("{:02}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60)
}

pub fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

pub fn fmt_clock(secs: i64) -> String {
    let s = secs.rem_euclid(86400);
    let (h, m) = (s / 3600, s % 3600 / 60);
    format!("{}:{:02} {}", if h % 12 == 0 { 12 } else { h % 12 }, m, if h < 12 { "AM" } else { "PM" })
}

fn nth_weekday(y: i64, m: i64, wd: usize, n: i64) -> i64 {
    let first = days_from_civil(y, m, 1);
    first + ((wd as i64 - weekday(first) as i64).rem_euclid(7)) + 7 * (n - 1)
}

fn easter(y: i64) -> i64 {
    let (a, b, c) = (y % 19, y / 100, y % 100);
    let (d, e) = (b / 4, b % 4);
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let mo = (h + l - 7 * m + 114) / 31;
    let da = (h + l - 7 * m + 114) % 31 + 1;
    days_from_civil(y, mo, da)
}

/// Fixed or computed holiday in a given year.
fn holiday(name: &str, y: i64) -> Option<i64> {
    Some(match name {
        "christmas" | "christmas day" | "xmas" => days_from_civil(y, 12, 25),
        "christmas eve" => days_from_civil(y, 12, 24),
        "new year" | "new years" | "new year's" | "new year's day" | "new years day" => {
            days_from_civil(y, 1, 1)
        }
        "new year's eve" | "new years eve" => days_from_civil(y, 12, 31),
        "halloween" => days_from_civil(y, 10, 31),
        "valentine" | "valentines" | "valentine's" | "valentines day" | "valentine's day" => {
            days_from_civil(y, 2, 14)
        }
        "independence day" | "july 4th" => days_from_civil(y, 7, 4),
        "april fools" | "april fool's day" => days_from_civil(y, 4, 1),
        "st patrick's day" | "st patricks day" => days_from_civil(y, 3, 17),
        "thanksgiving" => nth_weekday(y, 11, 3, 4),
        "easter" => easter(y),
        _ => return None,
    })
}

fn month_idx(s: &str) -> Option<i64> {
    if s.len() < 3 {
        return None;
    }
    MONTHS.iter().position(|m| m == &s || (s.len() == 3 && m.starts_with(s)) || (s == "sept" && *m == "september")).map(|i| i as i64 + 1)
}

fn parse_num(s: &str) -> Option<f64> {
    match s {
        "a" | "an" | "one" => Some(1.0),
        "two" => Some(2.0),
        "three" => Some(3.0),
        "four" => Some(4.0),
        "five" => Some(5.0),
        _ => s.parse().ok(),
    }
}

/// Parse a date phrase to days since epoch, relative to `today`.
pub fn parse_date(s: &str, today: i64) -> Option<i64> {
    let s = s.trim().trim_end_matches('?');
    match s {
        "today" | "now" => return Some(today),
        "tomorrow" => return Some(today + 1),
        "yesterday" => return Some(today - 1),
        _ => {}
    }
    let (ty, _, _) = civil_from_days(today);
    let next_after = |f: &dyn Fn(i64) -> Option<i64>| -> Option<i64> {
        let a = f(ty)?;
        if a >= today { Some(a) } else { f(ty + 1) }
    };
    if let Some(d) = next_after(&|y| holiday(s, y)) {
        return Some(d);
    }
    // weekdays
    let (kind, rest) = if let Some(r) = s.strip_prefix("next ") {
        (1, r)
    } else if let Some(r) = s.strip_prefix("last ") {
        (-1, r)
    } else if let Some(r) = s.strip_prefix("this ") {
        (0, r)
    } else {
        (2, s)
    };
    if let Some(wd) = WEEKDAYS.iter().position(|w| *w == rest || (rest.len() == 3 && w.starts_with(rest))) {
        let cur = weekday(today) as i64;
        let fwd = (wd as i64 - cur).rem_euclid(7);
        let back = (cur - wd as i64).rem_euclid(7);
        return Some(match kind {
            1 => today + if fwd == 0 { 7 } else { fwd },
            -1 => today - if back == 0 { 7 } else { back },
            _ => today + fwd,
        });
    }
    // ISO / numeric
    let iso = re!(r"^(\d{4})-(\d{1,2})-(\d{1,2})$");
    if let Some(c) = iso.captures(s) {
        return valid(c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?);
    }
    let us = re!(r"^(\d{1,2})/(\d{1,2})/(\d{4})$");
    if let Some(c) = us.captures(s) {
        return valid(c[3].parse().ok()?, c[1].parse().ok()?, c[2].parse().ok()?);
    }
    // "jan 1", "january 1st 2027", "1 jan", "25 dec 2026"
    let md = re!(r"^([a-z]+)\.? (\d{1,2})(?:st|nd|rd|th)?(?:,? (\d{4}))?$");
    if let Some(c) = md.captures(s) {
        let m = month_idx(&c[1])?;
        return ymd_or_next(c.get(3).and_then(|y| y.as_str().parse().ok()), m, c[2].parse().ok()?, today);
    }
    let dm = re!(r"^(\d{1,2})(?:st|nd|rd|th)? ([a-z]+)\.?(?:,? (\d{4}))?$");
    if let Some(c) = dm.captures(s) {
        let m = month_idx(&c[2])?;
        return ymd_or_next(c.get(3).and_then(|y| y.as_str().parse().ok()), m, c[1].parse().ok()?, today);
    }
    None
}

fn valid(y: i64, m: i64, d: i64) -> Option<i64> {
    if (1..=12).contains(&m) && d >= 1 && d <= month_len(y, m) {
        Some(days_from_civil(y, m, d))
    } else {
        None
    }
}

fn ymd_or_next(y: Option<i64>, m: i64, d: i64, today: i64) -> Option<i64> {
    if let Some(y) = y {
        return valid(y, m, d);
    }
    let (ty, _, _) = civil_from_days(today);
    match valid(ty, m, d) {
        Some(a) if a >= today => Some(a),
        _ => valid(ty + 1, m, d),
    }
}

/// Count weekdays in (a, b] when b>a, signed.
pub fn workdays_between(a: i64, b: i64) -> i64 {
    let (lo, hi, sign) = if b >= a { (a, b, 1) } else { (b, a, -1) };
    let mut n = 0;
    for d in lo + 1..=hi {
        if weekday(d) < 5 {
            n += 1;
        }
    }
    n * sign
}

fn add_workdays(start: i64, n: i64) -> i64 {
    let step = if n < 0 { -1 } else { 1 };
    let (mut d, mut left) = (start, n.abs());
    while left > 0 {
        d += step;
        if weekday(d) < 5 {
            left -= 1;
        }
    }
    d
}

fn plural(n: f64, unit: &str) -> String {
    let t = super::fmt_num(n);
    if (n - 1.0).abs() < 1e-9 { format!("{t} {unit}") } else { format!("{t} {unit}s") }
}

fn unit_kind(u: &str) -> Option<&'static str> {
    let u = u.trim_end_matches('s');
    Some(match u {
        "second" | "sec" => "second",
        "minute" | "min" => "minute",
        "hour" | "hr" => "hour",
        "day" => "day",
        "week" | "wk" => "week",
        "month" => "month",
        "year" | "yr" => "year",
        "workday" | "work day" | "business day" | "working day" | "weekday" => "workday",
        _ => return None,
    })
}

fn res(q: &str, answer: String, cq: &str, ca: String) -> Option<CalcResult> {
    Some(CalcResult {
        question: q.to_string(),
        answer,
        caption_question: cq.to_string(),
        caption_answer: ca,
    })
}

pub fn query(ctx: &Ctx, q: &str) -> Option<CalcResult> {
    let off = ctx.local_minutes(ctx.now) as i64 * 60;
    let local = ctx.now + off;
    let today = local.div_euclid(86400);

    if matches!(q, "now" | "today" | "tomorrow" | "yesterday" | "date" | "what day is it" | "what is the date" | "today's date") {
        let d = match q {
            "tomorrow" => today + 1,
            "yesterday" => today - 1,
            _ => today,
        };
        let ca = if q == "now" || q == "date" { fmt_clock(local) } else { "Date".into() };
        return res(q, fmt_date(d), "Date", ca);
    }

    // "<n> <unit> from now / ago / from <date>" and "in <n> <unit>"
    let rel = re!(r"^(?:in )?(\d+(?:\.\d+)?|an?|one|two|three|four|five) ([a-z ]+?)(?: (from now|from today|ago|later|hence|(?:from|after|before) (.+)))?$");
    if let Some(c) = rel.captures(q) {
        let has_in = q.starts_with("in ");
        let suffix = c.get(3).map(|m| m.as_str());
        if has_in || suffix.is_some() {
            let n = parse_num(&c[1])?;
            let kind = unit_kind(&c[2])?;
            let sign = match suffix {
                Some("ago") => -1.0,
                Some(s) if s.starts_with("before") => -1.0,
                _ => 1.0,
            };
            let base_days = match c.get(4) {
                Some(d) => parse_date(d.as_str(), today)?,
                None => match suffix {
                    Some(s) if s.starts_with("from tomorrow") => today + 1,
                    _ => today,
                },
            };
            let n = n * sign;
            let from_label = match (c.get(4), suffix) {
                (Some(d), _) => format!("From {}", d.as_str()),
                (None, Some("ago")) => "Before today".to_string(),
                _ => "From today".to_string(),
            };
            return match kind {
                "second" | "minute" | "hour" => {
                    let unit = match kind { "second" => 1.0, "minute" => 60.0, _ => 3600.0 };
                    let start = if c.get(4).is_some() { base_days * 86400 } else { local };
                    let t = start + (n * unit) as i64;
                    res(q, format!("{} {}", fmt_date(t.div_euclid(86400)), fmt_clock(t)), "Date and time", from_label)
                }
                "workday" => {
                    let d = add_workdays(base_days, n as i64);
                    res(q, fmt_date(d), "Work days", from_label)
                }
                _ => {
                    let d = match kind {
                        "day" => base_days + n as i64,
                        "week" => base_days + (n * 7.0) as i64,
                        "month" => add_months(base_days, n as i64),
                        _ => add_months(base_days, (n * 12.0) as i64),
                    };
                    res(q, fmt_date(d), "Date", from_label)
                }
            };
        }
    }

    // "<unit>s until/since <date>"
    let until = re!(r"^(?:how (?:many|long) )?([a-z ]+?) (until|till|to|since|from) (.+)$");
    if let Some(c) = until.captures(q) {
        if let (Some(kind), Some(target)) = (unit_kind(&c[1]), parse_date(&c[3], today)) {
            let raw = target - today;
            let diff = if &c[2] == "since" || &c[2] == "from" { -raw } else { raw };
            let cq = if diff >= 0 && (&c[2] == "since" || &c[2] == "from") { "Since" } else { "Until" };
            let (val, label) = match kind {
                "day" => (diff as f64, "day"),
                "week" => (diff as f64 / 7.0, "week"),
                "month" => (diff as f64 / 30.436875, "month"),
                "year" => (diff as f64 / 365.2425, "year"),
                "hour" => (diff as f64 * 24.0, "hour"),
                "minute" => (diff as f64 * 1440.0, "minute"),
                "second" => (diff as f64 * 86400.0, "second"),
                _ => (workdays_between(today, today + diff) as f64, "work day"),
            };
            let val = if kind == "day" || kind == "workday" { val } else { (val * 100.0).round() / 100.0 };
            return res(q, plural(val, label), cq, fmt_date(target));
        }
    }

    // "days between A and B"
    let btw = re!(r"^([a-z ]+?) between (.+) and (.+)$");
    if let Some(c) = btw.captures(q) {
        if let (Some(kind), Some(a), Some(b)) = (unit_kind(&c[1]), parse_date(&c[2], today), parse_date(&c[3], today)) {
            let diff = (b - a).abs();
            let (val, label) = match kind {
                "week" => (diff as f64 / 7.0, "week"),
                "workday" => (workdays_between(a.min(b), a.max(b)) as f64, "work day"),
                _ => (diff as f64, "day"),
            };
            let val = if label == "week" { (val * 100.0).round() / 100.0 } else { val };
            return res(q, plural(val, label), "Between", format!("{} and {}", fmt_date(a), fmt_date(b)));
        }
    }

    // "<date> + 3 weeks"
    let arith = re!(r"^(.+?) (\+|-|plus|minus) (\d+) ([a-z ]+)$");
    if let Some(c) = arith.captures(q) {
        if let (Some(base), Some(kind)) = (parse_date(&c[1], today), unit_kind(&c[4])) {
            let n: i64 = c[3].parse().ok()?;
            let n = if &c[2] == "-" || &c[2] == "minus" { -n } else { n };
            let d = match kind {
                "day" => base + n,
                "week" => base + 7 * n,
                "month" => add_months(base, n),
                "year" => add_months(base, 12 * n),
                "workday" => add_workdays(base, n),
                _ => return None,
            };
            return res(q, fmt_date(d), "Date", fmt_date(base));
        }
    }

    // "next friday", "christmas" with prefix forms
    if q.starts_with("next ") || q.starts_with("last ") || q.starts_with("this ") {
        if let Some(d) = parse_date(q, today) {
            let diff = d - today;
            return res(q, fmt_date(d), "Date", if diff >= 0 { format!("In {}", plural(diff as f64, "day")) } else { format!("{} ago", plural(-diff as f64, "day")) });
        }
    }
    unix(ctx, q, local, today)
}

pub fn fmt_utc(ts: i64) -> String {
    format!("{} {} UTC", {
        let (y, m, d) = civil_from_days(ts.div_euclid(86400));
        format!("{y}-{m:02}-{d:02}")
    }, fmt_hms(ts))
}

fn unix(ctx: &Ctx, q: &str, local: i64, today: i64) -> Option<CalcResult> {
    let to_date = |ts: i64| -> Option<CalcResult> {
        let off = ctx.local_minutes(ts) as i64 * 60;
        let l = ts + off;
        let (y, m, d) = civil_from_days(l.div_euclid(86400));
        res(
            q,
            fmt_utc(ts),
            "Unix time",
            format!("Local {y}-{m:02}-{d:02} {}", fmt_hms(l)),
        )
    };
    let a = re!(r"^(?:unix|unix time|timestamp|epoch)(?: (now|-?\d+))?$");
    if let Some(c) = a.captures(q) {
        return match c.get(1).map(|m| m.as_str()) {
            None | Some("now") => res(q, ctx.now.to_string(), "Unix time", fmt_utc(ctx.now)),
            Some(n) => to_date(norm_ts(n.parse().ok()?)),
        };
    }
    let b = re!(r"^(?:unix|timestamp|epoch) (?:to|in|as) (?:date|time|datetime)$");
    if b.is_match(q) {
        return to_date(ctx.now);
    }
    let c = re!(r"^(-?\d{9,13}) (?:to|in|as|into) (?:date|time|datetime|human)$");
    if let Some(m) = c.captures(q) {
        return to_date(norm_ts(m[1].parse().ok()?));
    }
    let d = re!(r"^(.+?) (?:to|in|as|into) (?:unix|unix time|timestamp|epoch)$");
    if let Some(m) = d.captures(q) {
        let s = &m[1];
        if s == "now" || s == "time" {
            return res(q, ctx.now.to_string(), "Now", fmt_utc(ctx.now));
        }
        let days = parse_date(s, today)?;
        let off = ctx.local_minutes(days * 86400) as i64 * 60;
        let ts = days * 86400 - off;
        let _ = local;
        return res(q, ts.to_string(), &fmt_date(days), "Local midnight".into());
    }
    None
}

fn norm_ts(n: i64) -> i64 {
    if n.abs() >= 100_000_000_000 { n / 1000 } else { n }
}

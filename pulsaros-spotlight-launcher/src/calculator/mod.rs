//! Raycast-style calculator: arithmetic, Soulver percentages, units, mixed units,
//! bases, dates, time zones, Unix time and currencies. Pure logic, no UI imports.

#![allow(dead_code, clippy::needless_range_loop, clippy::manual_range_contains, clippy::unnecessary_map_or)]

/// Compile a regex once per call site.
macro_rules! re {
    ($p:expr) => {{
        static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        R.get_or_init(|| regex::Regex::new($p).unwrap())
    }};
}

mod currency;
mod dates;
mod history;
mod tz;
mod units;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use currency::{refresh_currencies, CurrencyTable};
#[allow(unused_imports)]
pub use history::{CalcHistory, HistoryRow};

use units::{dim_text, lookup, Dim, Unit, CUR, NONE};

#[derive(Clone, Debug, PartialEq)]
pub struct CalcResult {
    pub question: String,
    pub answer: String,
    pub caption_question: String,
    pub caption_answer: String,
}

#[derive(Clone, Copy)]
pub enum LocalOff {
    Fixed(i32),
    System,
}

/// Everything the evaluator needs from the outside world (clock, local zone, rates).
pub struct Ctx<'a> {
    pub now: i64,
    pub local: LocalOff,
    pub cur: &'a CurrencyTable,
}

impl Ctx<'_> {
    pub fn local_minutes(&self, ts: i64) -> i32 {
        match self.local {
            LocalOff::Fixed(m) => m,
            LocalOff::System => {
                let t = ts as libc::time_t;
                let mut tm: libc::tm = unsafe { std::mem::zeroed() };
                unsafe { libc::localtime_r(&t, &mut tm) };
                (tm.tm_gmtoff / 60) as i32
            }
        }
    }
}

pub struct Calculator;

impl Calculator {
    /// Legacy API: (answer, "= question") for the search list.
    pub fn evaluate(query: &str) -> Option<(String, String)> {
        Self::calculate(query).map(|r| (r.answer, format!("= {}", r.question)))
    }

    /// Rich API for the two-pane card.
    pub fn calculate(query: &str) -> Option<CalcResult> {
        let cur = currency::current();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let ctx = Ctx { now, local: LocalOff::System, cur: &cur };
        calculate_with(&ctx, query)
    }
}

// ---------------------------------------------------------------- formatting

/// Up to ~10 significant digits, trailing zeros trimmed, thousands grouped.
pub fn fmt_num(x: f64) -> String {
    if !x.is_finite() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    let a = x.abs();
    if !(1e-7..1e15).contains(&a) {
        return format!("{:e}", x);
    }
    let mag = a.log10().floor() as i32;
    let dec = (9 - mag).clamp(0, 10) as usize;
    let mut s = format!("{:.*}", dec, x);
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if s == "-0" {
        return "0".into();
    }
    group(&s)
}

fn group(s: &str) -> String {
    let (sign, rest) = match s.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", s),
    };
    let (int, frac) = match rest.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (rest, None),
    };
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    match frac {
        Some(f) => format!("{sign}{out}.{f}"),
        None => format!("{sign}{out}"),
    }
}

fn money(x: f64, code: &str) -> String {
    let r = (x * 100.0).round() / 100.0;
    let body = if r == 0.0 && x != 0.0 { fmt_num(x) } else { group(&format!("{:.2}", r.abs())) };
    let sign = if r < 0.0 || (r == 0.0 && x < 0.0) { "-" } else { "" };
    match code {
        "USD" => format!("{sign}${body}"),
        "EUR" => format!("{sign}€{body}"),
        "GBP" => format!("{sign}£{body}"),
        "JPY" => format!("{sign}¥{body}"),
        c => format!("{sign}{body} {c}"),
    }
}

// ---------------------------------------------------------------- values

#[derive(Clone, Debug)]
struct Val {
    /// Value in coherent base units (or a plain number).
    n: f64,
    dim: Dim,
    /// Display units with exponents.
    disp: Vec<(Unit, i8)>,
    /// Literal percent (n holds the fraction).
    pct: bool,
    /// Mixed-unit display (e.g. ft + in), largest first.
    mixed: Vec<Unit>,
    base: u32,
}

impl Val {
    fn num(n: f64) -> Self {
        Val { n, dim: NONE, disp: vec![], pct: false, mixed: vec![], base: 0 }
    }

    fn from_disp(x: f64, disp: Vec<(Unit, i8)>) -> Self {
        let mut v = Val::num(0.0);
        v.dim = dim_from(&disp);
        v.disp = disp;
        v.set_disp_n(x);
        v
    }

    fn disp_factor(&self) -> f64 {
        self.disp.iter().map(|(u, e)| u.f.powi(*e as i32)).product()
    }

    fn single_offset(&self) -> Option<&Unit> {
        match self.disp.as_slice() {
            [(u, 1)] if u.off != 0.0 => Some(u),
            _ => None,
        }
    }

    fn disp_n(&self) -> f64 {
        match self.single_offset() {
            Some(u) => ((self.n / u.f - u.off) * 1e9).round() / 1e9,
            None => self.n / self.disp_factor(),
        }
    }

    fn set_disp_n(&mut self, x: f64) {
        self.n = match self.single_offset() {
            Some(u) => (x + u.off) * u.f,
            None => x * self.disp_factor(),
        };
    }

    /// Magnitude usable for +/-: temperatures contribute their delta.
    fn delta(&self) -> f64 {
        match self.single_offset() {
            Some(u) => self.n - u.off * u.f,
            None => self.n,
        }
    }
}

fn dim_from(disp: &[(Unit, i8)]) -> Dim {
    let mut d = NONE;
    for (u, e) in disp {
        for i in 0..8 {
            d[i] += u.dim[i] * e;
        }
    }
    d
}

fn merge(into: &mut Vec<(Unit, i8)>, u: &Unit, e: i8) {
    if let Some(i) = into.iter().position(|(x, _)| x.dim == u.dim && x.dim != NONE) {
        into[i].1 += e;
        if into[i].1 == 0 {
            into.remove(i);
        }
    } else if e != 0 {
        into.push((u.clone(), e));
    }
}

fn sup(e: i8) -> String {
    match e {
        1 => String::new(),
        2 => "²".into(),
        3 => "³".into(),
        e => format!("^{e}"),
    }
}

fn disp_text(d: &[(Unit, i8)]) -> String {
    let part = |sel: &dyn Fn(i8) -> bool| -> String {
        d.iter()
            .filter(|(_, e)| sel(*e))
            .map(|(u, e)| format!("{}{}", u.sym, sup(e.abs())))
            .collect::<Vec<_>>()
            .join("·")
    };
    let (num, den) = (part(&|e| e > 0), part(&|e| e < 0));
    match (num.is_empty(), den.is_empty()) {
        (_, true) => num,
        (true, false) => format!("1/{den}"),
        _ => format!("{num}/{den}"),
    }
}

fn long_text(d: &[(Unit, i8)]) -> String {
    match d {
        [(u, 1)] => u.long.clone(),
        _ => disp_text(d),
    }
}

fn fmt_val(v: &Val) -> String {
    if v.base != 0 {
        let i = v.n.round() as i64;
        return match v.base {
            16 => format!("0x{:X}", i),
            2 => format!("0b{:b}", i),
            8 => format!("0o{:o}", i),
            _ => fmt_num(v.n),
        };
    }
    if v.pct {
        return format!("{}%", fmt_num(v.n * 100.0));
    }
    if !v.mixed.is_empty() {
        let neg = v.n < 0.0;
        let mut rest = v.n.abs();
        let mut parts = Vec::new();
        for (i, u) in v.mixed.iter().enumerate() {
            if i + 1 == v.mixed.len() {
                let x = rest / u.f;
                if parts.is_empty() || fmt_num(x) != "0" {
                    parts.push(format!("{} {}", fmt_num(x), u.sym));
                }
            } else {
                let q = (rest / u.f + 1e-9).floor();
                rest -= q * u.f;
                if q != 0.0 {
                    parts.push(format!("{} {}", fmt_num(q), u.sym));
                }
            }
        }
        return format!("{}{}", if neg { "-" } else { "" }, parts.join(" "));
    }
    if v.disp.is_empty() {
        return if v.dim == NONE { fmt_num(v.n) } else { format!("{} {}", fmt_num(v.n), dim_text(&v.dim)) };
    }
    let x = v.disp_n();
    if let [(u, 1)] = v.disp.as_slice() {
        if u.dim[CUR] == 1 && u.dim.iter().sum::<i8>() == 1 {
            return money(x, &u.sym);
        }
    }
    format!("{} {}", fmt_num(x), disp_text(&v.disp))
}

// ---------------------------------------------------------------- tokens

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Id(String),
    Op(char),
    LP,
    RP,
    Comma,
    Cur(&'static str),
}

fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let c: Vec<char> = s.chars().collect();
    let mut t = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        match ch {
            ' ' | '\t' | '=' => i += 1,
            '0'..='9' | '.' if ch != '.' || c.get(i + 1).map_or(false, |d| d.is_ascii_digit()) => {
                if ch == '0' && matches!(c.get(i + 1), Some('x' | 'b' | 'o')) {
                    let radix = match c[i + 1] { 'x' => 16, 'b' => 2, _ => 8 };
                    let mut j = i + 2;
                    while j < c.len() && c[j].is_digit(radix) {
                        j += 1;
                    }
                    if j > i + 2 {
                        let digits: String = c[i + 2..j].iter().collect();
                        t.push(Tok::Num(i64::from_str_radix(&digits, radix).ok()? as f64));
                        i = j;
                        continue;
                    }
                }
                let mut j = i;
                while j < c.len() && (c[j].is_ascii_digit() || c[j] == '.') {
                    j += 1;
                }
                if j < c.len() && c[j] == 'e' {
                    let mut k = j + 1;
                    if matches!(c.get(k), Some('+' | '-')) {
                        k += 1;
                    }
                    if c.get(k).map_or(false, |d| d.is_ascii_digit()) {
                        while k < c.len() && c[k].is_ascii_digit() {
                            k += 1;
                        }
                        j = k;
                    }
                }
                let text: String = c[i..j].iter().collect();
                let mut n: f64 = text.parse().ok()?;
                // "5k" = 5000 (attached only)
                if c.get(j) == Some(&'k') && !c.get(j + 1).map_or(false, |d| d.is_alphanumeric()) {
                    n *= 1000.0;
                    j += 1;
                }
                t.push(Tok::Num(n));
                i = j;
            }
            '$' => { t.push(Tok::Cur("usd")); i += 1 }
            '€' => { t.push(Tok::Cur("eur")); i += 1 }
            '£' => { t.push(Tok::Cur("gbp")); i += 1 }
            '¥' => { t.push(Tok::Cur("jpy")); i += 1 }
            '°' => {
                let nx = c.get(i + 1).copied();
                if matches!(nx, Some('c' | 'f' | 'k')) && !c.get(i + 2).map_or(false, |d| d.is_alphabetic()) {
                    t.push(Tok::Id(nx.unwrap().to_string()));
                    i += 2;
                } else {
                    t.push(Tok::Id("deg".into()));
                    i += 1;
                }
            }
            '²' => { t.push(Tok::Op('^')); t.push(Tok::Num(2.0)); i += 1 }
            '³' => { t.push(Tok::Op('^')); t.push(Tok::Num(3.0)); i += 1 }
            '"' => { t.push(Tok::Id("in".into())); i += 1 }
            '\'' => { t.push(Tok::Id("ft".into())); i += 1 }
            '+' | '-' | '/' | '%' | '!' => { t.push(Tok::Op(ch)); i += 1 }
            '−' => { t.push(Tok::Op('-')); i += 1 }
            '×' | '·' => { t.push(Tok::Op('*')); i += 1 }
            '÷' => { t.push(Tok::Op('/')); i += 1 }
            '^' => { t.push(Tok::Op('^')); i += 1 }
            '*' => {
                if c.get(i + 1) == Some(&'*') {
                    t.push(Tok::Op('^'));
                    i += 2;
                } else {
                    t.push(Tok::Op('*'));
                    i += 1;
                }
            }
            '(' => { t.push(Tok::LP); i += 1 }
            ')' => { t.push(Tok::RP); i += 1 }
            ',' => { t.push(Tok::Comma); i += 1 }
            'π' => { t.push(Tok::Id("pi".into())); i += 1 }
            c0 if c0.is_alphabetic() || c0 == '_' => {
                let mut j = i;
                while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
                    j += 1;
                }
                t.push(Tok::Id(c[i..j].iter().collect()));
                i = j;
            }
            _ => return None,
        }
    }
    Some(t)
}

/// Lowercase, spoken operators to symbols, thousands separators stripped.
fn normalize(raw: &str) -> String {
    let mut s = format!(" {} ", raw.trim().to_lowercase());
    for (a, b) in [
        (" plus ", " + "),
        (" minus ", " - "),
        (" times ", " * "),
        (" multiplied by ", " * "),
        (" divided by ", " / "),
        (" over ", " / "),
        (" squared ", "^2 "),
        (" cubed ", "^3 "),
        ("fl oz", "floz"),
        ("fluid ounce", "floz"),
        (" and ", " "),
    ] {
        if a == " and " {
            continue;
        }
        s = s.replace(a, b);
    }
    let mut s = s.trim().to_string();
    for p in ["what is ", "what's ", "whats ", "calculate ", "calc ", "convert ", "how much is "] {
        if let Some(r) = s.strip_prefix(p) {
            s = r.to_string();
            break;
        }
    }
    s = s.trim_end_matches(['?', '=']).trim().to_string();
    if !s.contains('(') {
        let c: Vec<char> = s.chars().collect();
        let mut out = String::new();
        for (i, &ch) in c.iter().enumerate() {
            let is_sep = ch == ','
                && i > 0
                && c[i - 1].is_ascii_digit()
                && c.get(i + 1..i + 4).map_or(false, |g| g.iter().all(|d| d.is_ascii_digit()))
                && !c.get(i + 4).map_or(false, |d| d.is_ascii_digit());
            if !is_sep {
                out.push(ch);
            }
        }
        s = out;
    }
    s
}

// ---------------------------------------------------------------- parser

type R<T> = Result<T, ()>;

struct Parser<'a> {
    t: Vec<Tok>,
    pos: usize,
    ctx: &'a Ctx<'a>,
    ops: u32,
    src_cap: Option<String>,
    dst_cap: Option<String>,
}

const FUNCS: &[&str] = &[
    "sqrt", "cbrt", "abs", "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh",
    "ln", "log", "log2", "log10", "exp", "floor", "ceil", "round", "trunc", "fact", "factorial",
    "sign", "min", "max", "sum", "avg", "pow", "hypot", "mod", "radians", "degrees",
];

fn is_const(s: &str) -> bool {
    matches!(s, "pi" | "tau" | "phi" | "e")
}

fn scale_word(s: &str) -> Option<f64> {
    Some(match s {
        "thousand" => 1e3,
        "million" => 1e6,
        "billion" => 1e9,
        "trillion" => 1e12,
        "dozen" => 12.0,
        "gross" => 144.0,
        "score" => 20.0,
        _ => return None,
    })
}

fn factorial(n: f64) -> R<f64> {
    if n < 0.0 || n.fract() != 0.0 || n > 170.0 {
        return Err(());
    }
    Ok((1..=n as u64).fold(1.0, |a, b| a * b as f64))
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.pos)
    }
    fn peek_at(&self, k: usize) -> Option<&Tok> {
        self.t.get(self.pos + k)
    }
    fn is_id(&self, k: usize, s: &str) -> bool {
        matches!(self.peek_at(k), Some(Tok::Id(x)) if x == s)
    }

    fn statement(&mut self) -> R<Val> {
        let mut v = self.add()?;
        if let Some(Tok::Id(k)) = self.peek() {
            if matches!(k.as_str(), "to" | "into" | "as" | "in") {
                self.pos += 1;
                v = self.convert(v)?;
                self.ops += 1;
            }
        }
        Ok(v)
    }

    fn convert(&mut self, mut v: Val) -> R<Val> {
        self.src_cap = Some(if !v.disp.is_empty() { long_text(&v.disp) } else { "Number".into() });
        if let Some(Tok::Id(k)) = self.peek() {
            let base = match k.as_str() {
                "hex" | "hexadecimal" => Some((16, "Hexadecimal")),
                "bin" | "binary" => Some((2, "Binary")),
                "oct" | "octal" => Some((8, "Octal")),
                "dec" | "decimal" => Some((10, "Decimal")),
                _ => None,
            };
            if let Some((b, name)) = base {
                self.pos += 1;
                if v.dim != NONE || v.n.fract().abs() > 1e-9 {
                    return Err(());
                }
                v.base = b;
                v.pct = false;
                self.dst_cap = Some(name.into());
                return Ok(v);
            }
            if matches!(k.as_str(), "percent" | "percentage") {
                self.pos += 1;
                self.dst_cap = Some("Percent".into());
                v.pct = true;
                return Ok(v);
            }
        }
        if self.peek() == Some(&Tok::Op('%')) {
            self.pos += 1;
            self.dst_cap = Some("Percent".into());
            v.pct = true;
            return Ok(v);
        }
        let mut targets: Vec<Vec<(Unit, i8)>> = Vec::new();
        while self.pos < self.t.len() {
            targets.push(self.unit_expr().ok_or(())?);
        }
        if targets.is_empty() {
            return Err(());
        }
        let dim = dim_from(&targets[0]);
        if dim != v.dim || dim == NONE {
            return Err(());
        }
        if targets.len() == 1 {
            self.dst_cap = Some(long_text(&targets[0]));
            v.disp = targets.remove(0);
            v.mixed.clear();
        } else {
            let mut us = Vec::new();
            for t in &targets {
                match t.as_slice() {
                    [(u, 1)] if u.dim == dim && u.off == 0.0 => us.push(u.clone()),
                    _ => return Err(()),
                }
            }
            us.sort_by(|a, b| b.f.partial_cmp(&a.f).unwrap());
            self.dst_cap = Some(us.iter().map(|u| u.long.clone()).collect::<Vec<_>>().join(" + "));
            v.disp = vec![(us.last().unwrap().clone(), 1)];
            v.mixed = us;
        }
        v.pct = false;
        v.base = 0;
        Ok(v)
    }

    fn add(&mut self) -> R<Val> {
        let mut l = self.mul()?;
        loop {
            let sub = match self.peek() {
                Some(Tok::Op('+')) => false,
                Some(Tok::Op('-')) => true,
                _ => return Ok(l),
            };
            self.pos += 1;
            let r = self.mul()?;
            self.ops += 1;
            l = if r.pct && !l.pct {
                let f = if sub { 1.0 - r.n } else { 1.0 + r.n };
                let mut o = l;
                o.n *= f;
                o.mixed.clear();
                o
            } else {
                add_vals(l, r, sub)?
            };
        }
    }

    fn pct_postfix(&self) -> bool {
        // `%` at self.pos; percent unless an operand follows (then it is modulo).
        match self.peek_at(1) {
            None | Some(Tok::RP) | Some(Tok::Comma) | Some(Tok::Op(_)) => true,
            Some(Tok::Id(k)) => matches!(k.as_str(), "of" | "off" | "on" | "as" | "to" | "in" | "into"),
            _ => false,
        }
    }

    fn mul(&mut self) -> R<Val> {
        let mut l = self.unary()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::Op('*')) => {
                    self.pos += 1;
                    let r = self.unary()?;
                    self.ops += 1;
                    l = mul_vals(&l, &r, false)?;
                }
                Some(Tok::Op('/')) => {
                    self.pos += 1;
                    let r = self.unary()?;
                    self.ops += 1;
                    l = mul_vals(&l, &r, true)?;
                }
                Some(Tok::Op('%')) => {
                    self.pos += 1;
                    let r = self.unary()?;
                    self.ops += 1;
                    l = modulo(l, r)?;
                }
                Some(Tok::Id(k)) if k == "mod" => {
                    self.pos += 1;
                    let r = self.unary()?;
                    self.ops += 1;
                    l = modulo(l, r)?;
                }
                Some(Tok::Id(k)) if l.pct && matches!(k.as_str(), "of" | "off" | "on") => {
                    self.pos += 1;
                    let r = self.unary()?;
                    self.ops += 1;
                    let mut o = r;
                    o.n *= match k.as_str() {
                        "of" => l.n,
                        "off" => 1.0 - l.n,
                        _ => 1.0 + l.n,
                    };
                    o.mixed.clear();
                    o.pct = false;
                    l = o;
                }
                Some(Tok::LP) => {
                    let r = self.unary()?;
                    self.ops += 1;
                    l = mul_vals(&l, &r, false)?;
                }
                Some(Tok::Id(k)) if (is_const(&k) && k.len() > 1) || FUNCS.contains(&k.as_str()) => {
                    let r = self.unary()?;
                    self.ops += 1;
                    l = mul_vals(&l, &r, false)?;
                }
                _ => return Ok(l),
            }
        }
    }

    fn unary(&mut self) -> R<Val> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                let mut v = self.unary()?;
                if v.single_offset().is_some() {
                    v.set_disp_n(-v.disp_n());
                } else {
                    v.n = -v.n;
                }
                Ok(v)
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> R<Val> {
        let b = self.postfix()?;
        if self.peek() == Some(&Tok::Op('^')) {
            self.pos += 1;
            let e = self.unary()?;
            self.ops += 1;
            return pow_vals(b, e);
        }
        Ok(b)
    }

    fn postfix(&mut self) -> R<Val> {
        let mut v = self.primary()?;
        loop {
            match self.peek() {
                Some(Tok::Op('!')) => {
                    self.pos += 1;
                    self.ops += 1;
                    v = Val::num(factorial(v.n)?);
                }
                Some(Tok::Op('%')) if self.pct_postfix() && v.dim == NONE => {
                    self.pos += 1;
                    v.n /= 100.0;
                    v.pct = true;
                }
                _ => return Ok(v),
            }
        }
    }

    fn scales(&mut self, mut n: f64) -> f64 {
        while let Some(Tok::Id(k)) = self.peek() {
            match scale_word(k) {
                Some(m) => {
                    n *= m;
                    self.pos += 1;
                }
                None => break,
            }
        }
        n
    }

    fn primary(&mut self) -> R<Val> {
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                let n = self.scales(n);
                self.quantity(n)
            }
            Some(Tok::Cur(code)) => {
                self.pos += 1;
                let Some(Tok::Num(n)) = self.peek().cloned() else { return Err(()) };
                self.pos += 1;
                let n = self.scales(n);
                let u = lookup(code, self.ctx.cur).ok_or(())?;
                Ok(Val::from_disp(n, vec![(u, 1)]))
            }
            Some(Tok::LP) => {
                self.pos += 1;
                let v = self.add()?;
                if self.peek() != Some(&Tok::RP) {
                    return Err(());
                }
                self.pos += 1;
                Ok(v)
            }
            Some(Tok::Id(k)) => {
                if is_const(&k) && self.peek_at(1) != Some(&Tok::LP) {
                    self.pos += 1;
                    if k.len() > 1 {
                        self.ops += 1;
                    }
                    return Ok(Val::num(match k.as_str() {
                        "pi" => std::f64::consts::PI,
                        "tau" => std::f64::consts::TAU,
                        "phi" => std::f64::consts::GOLDEN_RATIO,
                        _ => std::f64::consts::E,
                    }));
                }
                if FUNCS.contains(&k.as_str()) {
                    self.pos += 1;
                    self.ops += 1;
                    let args = if self.peek() == Some(&Tok::LP) {
                        self.pos += 1;
                        let mut a = vec![self.add()?];
                        while self.peek() == Some(&Tok::Comma) {
                            self.pos += 1;
                            a.push(self.add()?);
                        }
                        if self.peek() != Some(&Tok::RP) {
                            return Err(());
                        }
                        self.pos += 1;
                        a
                    } else {
                        vec![self.unary()?]
                    };
                    return call(&k, args);
                }
                Err(())
            }
            _ => Err(()),
        }
    }

    /// A number optionally followed by units (and mixed continuations).
    fn quantity(&mut self, n: f64) -> R<Val> {
        let Some(first) = self.unit_expr() else { return Ok(Val::num(n)) };
        let mut v = Val::from_disp(n, first);
        let single = match v.disp.as_slice() {
            [(u, 1)] if u.dim[CUR] == 0 && u.off == 0.0 => Some(u.clone()),
            _ => None,
        };
        if let Some(u0) = single {
            let mut mixed = vec![u0.clone()];
            loop {
                let save = self.pos;
                let Some(Tok::Num(m)) = self.peek().cloned() else { break };
                self.pos += 1;
                match self.unit_expr().as_deref() {
                    Some([(u, 1)]) if u.dim == u0.dim && !mixed.iter().any(|x| x.sym == u.sym) => {
                        v.n += m * u.f;
                        mixed.push(u.clone());
                    }
                    _ => {
                        self.pos = save;
                        break;
                    }
                }
            }
            if mixed.len() > 1 {
                mixed.sort_by(|a, b| b.f.partial_cmp(&a.f).unwrap());
                v.mixed = mixed;
            }
        }
        Ok(v)
    }

    fn unit_expr(&mut self) -> Option<Vec<(Unit, i8)>> {
        let mut out = self.unit_atom()?;
        loop {
            let (mul, skip) = match self.peek() {
                Some(Tok::Op('/')) => (-1i8, 1),
                Some(Tok::Op('*')) => (1, 1),
                Some(Tok::Id(k)) if k == "per" => (-1, 1),
                _ => break,
            };
            let save = self.pos;
            self.pos += skip;
            match self.unit_atom() {
                Some(d) => {
                    for (u, e) in d {
                        merge(&mut out, &u, e * mul);
                    }
                }
                None => {
                    self.pos = save;
                    break;
                }
            }
        }
        Some(out)
    }

    fn unit_atom(&mut self) -> Option<Vec<(Unit, i8)>> {
        let save = self.pos;
        let mut exp = 1i8;
        if let Some(Tok::Id(s)) = self.peek() {
            match s.as_str() {
                "sq" | "square" => { exp = 2; self.pos += 1 }
                "cubic" => { exp = 3; self.pos += 1 }
                _ => {}
            }
        }
        let name = match self.peek() {
            Some(Tok::Id(s)) => s.clone(),
            _ => {
                self.pos = save;
                return None;
            }
        };
        if name == "in" {
            if let Some(Tok::Id(nx)) = self.peek_at(1) {
                if lookup(nx, self.ctx.cur).is_some()
                    || matches!(nx.as_str(), "hex" | "bin" | "oct" | "dec" | "binary" | "hexadecimal" | "octal" | "decimal")
                {
                    self.pos = save;
                    return None;
                }
            }
        }
        let Some(u) = lookup(&name, self.ctx.cur) else {
            self.pos = save;
            return None;
        };
        self.pos += 1;
        if self.peek() == Some(&Tok::Op('^')) {
            if let Some(Tok::Num(e)) = self.peek_at(1) {
                if e.fract() == 0.0 && e.abs() < 6.0 {
                    exp *= *e as i8;
                    self.pos += 2;
                }
            }
        }
        let parts: Vec<(Unit, i8)> = match u.expand {
            Some(ex) => {
                let (a, b) = ex.split_once('/')?;
                vec![(lookup(a, self.ctx.cur)?, exp), (lookup(b, self.ctx.cur)?, -exp)]
            }
            None => vec![(u, exp)],
        };
        Some(parts)
    }
}

fn add_vals(mut a: Val, mut b: Val, sub: bool) -> R<Val> {
    if a.dim == NONE && b.dim != NONE {
        // scalar adopts the other side's display unit
        a.disp = b.disp.clone();
        a.dim = b.dim;
        a.mixed = b.mixed.clone();
        a.n *= a.disp_factor();
    } else if b.dim == NONE && a.dim != NONE {
        b.n *= a.disp_factor();
        b.dim = a.dim;
    } else if a.dim != b.dim {
        return Err(());
    }
    let d = if b.dim == a.dim && b.single_offset().is_some() { b.delta() } else { b.n };
    a.n += if sub { -d } else { d };
    a.pct = a.pct && b.pct;
    a.base = 0;
    Ok(a)
}

fn mul_vals(a: &Val, b: &Val, div: bool) -> R<Val> {
    let s: i8 = if div { -1 } else { 1 };
    if div && b.n == 0.0 {
        return Err(());
    }
    let mut disp = a.disp.clone();
    for (u, e) in &b.disp {
        merge(&mut disp, u, e * s);
    }
    let mut dim = a.dim;
    for i in 0..8 {
        dim[i] += b.dim[i] * s;
    }
    if dim == NONE {
        disp.clear();
    }
    let mut v = Val::num(if div { a.n / b.n } else { a.n * b.n });
    v.dim = dim;
    v.disp = disp;
    Ok(v)
}

fn modulo(a: Val, b: Val) -> R<Val> {
    if a.dim != b.dim || b.n == 0.0 {
        return Err(());
    }
    let mut o = a;
    o.n %= b.n;
    o.pct = false;
    Ok(o)
}

fn pow_vals(b: Val, e: Val) -> R<Val> {
    if e.dim != NONE {
        return Err(());
    }
    let mut o = b;
    let k = e.n;
    if o.dim != NONE {
        if k.fract() != 0.0 || k.abs() > 5.0 {
            return Err(());
        }
        for d in o.dim.iter_mut() {
            *d *= k as i8;
        }
        for (_, x) in o.disp.iter_mut() {
            *x *= k as i8;
        }
    }
    o.n = o.n.powf(k);
    o.mixed.clear();
    o.pct = false;
    if o.n.is_nan() { Err(()) } else { Ok(o) }
}

fn call(name: &str, args: Vec<Val>) -> R<Val> {
    let a0 = args.first().ok_or(())?;
    // Functions that preserve the unit apply to the displayed number.
    if matches!(name, "abs" | "floor" | "ceil" | "trunc" | "round" | "sign") && a0.dim != NONE {
        let mut v = a0.clone();
        let x = v.disp_n();
        let r = match name {
            "abs" => x.abs(),
            "floor" => x.floor(),
            "ceil" => x.ceil(),
            "trunc" => x.trunc(),
            "round" => x.round(),
            _ => x.signum(),
        };
        v.mixed.clear();
        v.set_disp_n(r);
        return Ok(v);
    }
    if matches!(name, "min" | "max" | "sum" | "avg") && args.iter().all(|a| a.dim == a0.dim) {
        let mut acc = a0.clone();
        let ns: Vec<f64> = args.iter().map(|a| a.n).collect();
        acc.n = match name {
            "min" => ns.iter().cloned().fold(f64::INFINITY, f64::min),
            "max" => ns.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            "sum" => ns.iter().sum(),
            _ => ns.iter().sum::<f64>() / ns.len() as f64,
        };
        acc.mixed.clear();
        acc.pct = false;
        return Ok(acc);
    }
    if name == "sqrt" && a0.dim != NONE {
        if a0.dim.iter().any(|d| d % 2 != 0) || a0.n < 0.0 {
            return Err(());
        }
        let mut v = a0.clone();
        v.n = v.n.sqrt();
        for d in v.dim.iter_mut() { *d /= 2 }
        for (_, x) in v.disp.iter_mut() { *x /= 2 }
        v.disp.retain(|(_, x)| *x != 0);
        v.mixed.clear();
        return Ok(v);
    }
    if name == "cbrt" && a0.dim != NONE {
        if a0.dim.iter().any(|d| d % 3 != 0) {
            return Err(());
        }
        let mut v = a0.clone();
        v.n = v.n.cbrt();
        for d in v.dim.iter_mut() { *d /= 3 }
        for (_, x) in v.disp.iter_mut() { *x /= 3 }
        v.mixed.clear();
        return Ok(v);
    }
    // Everything else wants plain numbers (angles are already radians in base).
    let n = |i: usize| -> R<f64> {
        let v = args.get(i).ok_or(())?;
        if v.dim != NONE && !(v.dim == units::dim_of("angle") && matches!(name, "sin" | "cos" | "tan")) {
            return Err(());
        }
        Ok(v.n)
    };
    let x = n(0)?;
    let r = match name {
        "sqrt" => if x < 0.0 { return Err(()) } else { x.sqrt() },
        "cbrt" => x.cbrt(),
        "abs" => x.abs(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "asin" => x.asin(),
        "acos" => x.acos(),
        "atan" => x.atan(),
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "tanh" => x.tanh(),
        "ln" => x.ln(),
        "log" | "log10" => {
            if args.len() > 1 { x.ln() / n(1)?.ln() } else { x.log10() }
        }
        "log2" => x.log2(),
        "exp" => x.exp(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "trunc" => x.trunc(),
        "round" => {
            if args.len() > 1 {
                let p = 10f64.powi(n(1)? as i32);
                (x * p).round() / p
            } else {
                x.round()
            }
        }
        "fact" | "factorial" => factorial(x)?,
        "sign" => x.signum(),
        "min" => (0..args.len()).try_fold(f64::INFINITY, |a, i| n(i).map(|v| a.min(v)))?,
        "max" => (0..args.len()).try_fold(f64::NEG_INFINITY, |a, i| n(i).map(|v| a.max(v)))?,
        "sum" => (0..args.len()).try_fold(0.0, |a, i| n(i).map(|v| a + v))?,
        "avg" => (0..args.len()).try_fold(0.0, |a, i| n(i).map(|v| a + v))? / args.len() as f64,
        "pow" => x.powf(n(1)?),
        "hypot" => x.hypot(n(1)?),
        "mod" => {
            let m = n(1)?;
            if m == 0.0 { return Err(()) }
            x % m
        }
        "radians" => x.to_radians(),
        "degrees" => x.to_degrees(),
        _ => return Err(()),
    };
    if r.is_nan() || r.is_infinite() {
        return Err(());
    }
    Ok(Val::num(r))
}

// ---------------------------------------------------------------- driver

struct Evaluated {
    val: Val,
    ops: u32,
    src_cap: Option<String>,
    dst_cap: Option<String>,
}

fn eval_str(ctx: &Ctx, s: &str) -> R<Evaluated> {
    let t = tokenize(s).ok_or(())?;
    if t.is_empty() {
        return Err(());
    }
    let mut p = Parser { t, pos: 0, ctx, ops: 0, src_cap: None, dst_cap: None };
    let val = p.statement()?;
    if p.pos != p.t.len() || val.n.is_nan() || val.n.is_infinite() {
        return Err(());
    }
    Ok(Evaluated { val, ops: p.ops, src_cap: p.src_cap, dst_cap: p.dst_cap })
}

fn percent_of(ctx: &Ctx, q: &str) -> Option<CalcResult> {
    let mk = |ans: String, cq: &str, ca: &str| CalcResult {
        question: q.to_string(),
        answer: ans,
        caption_question: cq.to_string(),
        caption_answer: ca.to_string(),
    };
    let ratio = |a: &str, b: &str| -> Option<CalcResult> {
        let (a, b) = (eval_str(ctx, a).ok()?.val, eval_str(ctx, b).ok()?.val);
        if a.dim != b.dim || b.n == 0.0 {
            return None;
        }
        Some(mk(format!("{}%", fmt_num(a.n / b.n * 100.0)), "Percentage", "Percent"))
    };
    if let Some(c) = re!(r"^(.+?) as (?:an? )?(?:%|percent|percentage) of (.+)$").captures(q) {
        return ratio(&c[1], &c[2]);
    }
    if let Some(c) = re!(r"^(.+?) is what (?:%|percent|percentage) of (.+)$").captures(q) {
        return ratio(&c[1], &c[2]);
    }
    let whole = |part: &str, pct: &str| -> Option<CalcResult> {
        let part = eval_str(ctx, part).ok()?.val;
        let p = eval_str(ctx, &format!("{pct}%")).ok()?.val;
        if !p.pct || p.n == 0.0 {
            return None;
        }
        let mut w = part;
        w.n /= p.n;
        w.mixed.clear();
        Some(mk(fmt_val(&w), "Percentage", "Whole"))
    };
    if let Some(c) = re!(r"^(.+?) is (\d+(?:\.\d+)?)% of what$").captures(q) {
        return whole(&c[1], &c[2]);
    }
    if let Some(c) = re!(r"^(\d+(?:\.\d+)?)% of what is (.+)$").captures(q) {
        return whole(&c[2], &c[1]);
    }
    None
}

pub fn calculate_with(ctx: &Ctx, query: &str) -> Option<CalcResult> {
    let raw = query.trim();
    if raw.is_empty() || raw.len() > 200 {
        return None;
    }
    let q = normalize(raw);
    if q.is_empty() {
        return None;
    }
    let has_letter = q.chars().any(|c| c.is_alphabetic());
    if has_letter {
        if let Some(r) = dates::query(ctx, &q) {
            return Some(CalcResult { question: raw.to_string(), ..r });
        }
        if let Some(r) = tz::query(ctx, &q) {
            return Some(CalcResult { question: raw.to_string(), ..r });
        }
        if let Some(r) = percent_of(ctx, &q) {
            return Some(CalcResult { question: raw.to_string(), ..r });
        }
    }
    let ev = eval_str(ctx, &q).ok()?;
    if ev.ops == 0 {
        return None;
    }
    let v = &ev.val;
    let answer = fmt_val(v);
    let cq = ev.src_cap.unwrap_or_else(|| "Expression".to_string());
    let ca = ev.dst_cap.unwrap_or_else(|| {
        if v.pct {
            "Percent".into()
        } else if !v.mixed.is_empty() {
            v.mixed.iter().map(|u| u.long.clone()).collect::<Vec<_>>().join(" + ")
        } else if !v.disp.is_empty() {
            long_text(&v.disp)
        } else {
            "Result".into()
        }
    });
    Some(CalcResult { question: raw.to_string(), answer, caption_question: cq, caption_answer: ca })
}

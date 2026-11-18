use super::dates::days_from_civil;
use super::*;
use std::collections::HashMap;

/// Sat 2026-10-24 12:00:00 UTC; local zone fixed at UTC-4 (08:00 Sat).
fn now() -> i64 {
    days_from_civil(2026, 10, 24) * 86400 + 12 * 3600
}

fn table() -> CurrencyTable {
    let mut r = HashMap::new();
    for (k, v) in [("EUR", 0.8), ("GBP", 0.75), ("JPY", 150.0), ("CAD", 1.25)] {
        r.insert(k.to_string(), v);
    }
    CurrencyTable::from_usd_base(&r, "2026-10-23", 1)
}

fn calc(q: &str) -> Option<CalcResult> {
    let t = table();
    let ctx = Ctx { now: now(), local: LocalOff::Fixed(-240), cur: &t };
    calculate_with(&ctx, q)
}

fn run(cases: &[(&str, &str)]) {
    let mut bad = Vec::new();
    for (q, want) in cases {
        let got = calc(q).map(|r| r.answer);
        let ok = match (&got, *want) {
            (None, "None") => true,
            (Some(g), w) => g == w,
            _ => false,
        };
        if !ok {
            bad.push(format!("{q:?}: want {want:?}, got {got:?}"));
        }
    }
    assert!(bad.is_empty(), "{} failures:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn arithmetic() {
    run(&[
        ("2+2", "4"),
        ("2 + 3 * 4", "14"),
        ("(2+3)*4", "20"),
        ("10 / 4", "2.5"),
        ("2^10", "1,024"),
        ("2**3", "8"),
        ("-5 + 2", "-3"),
        ("10 mod 3", "1"),
        ("10 % 3", "1"),
        ("7 / 2 * 2", "7"),
        ("sqrt(16)", "4"),
        ("sqrt 81", "9"),
        ("abs(-5)", "5"),
        ("round(3.14159, 2)", "3.14"),
        ("floor(2.7)", "2"),
        ("ceil(2.1)", "3"),
        ("log10(1000)", "3"),
        ("log2(8)", "3"),
        ("ln(e)", "1"),
        ("5!", "120"),
        ("pi * 2", "6.283185307"),
        ("pi", "3.141592654"),
        ("max(3, 9, 4)", "9"),
        ("min(3, 9, 4)", "3"),
        ("sum(1, 2, 3)", "6"),
        ("avg(2, 4, 6)", "4"),
        ("1,000 + 500", "1,500"),
        ("1e3 * 2", "2,000"),
        ("0.1 + 0.2", "0.3"),
        ("100 / 3", "33.33333333"),
        ("1/3", "0.3333333333"),
        ("5k * 2", "10,000"),
        ("2 million * 3", "6,000,000"),
        ("sin(0)", "0"),
        ("cos(0)", "1"),
        ("sin(90 deg)", "1"),
        ("10 plus 5", "15"),
        ("10 times 5", "50"),
        ("20 divided by 4", "5"),
        ("what is 6 * 7", "42"),
        ("0xff + 1", "256"),
        ("2^-1", "0.5"),
        ("3 squared", "9"),
        ("2 × 3", "6"),
        ("9 ÷ 3", "3"),
        ("2(3+4)", "14"),
        ("-(2+3)", "-5"),
        ("2^3^2", "512"),
        ("pow(2, 8)", "256"),
        ("cbrt(27)", "3"),
        ("hypot(3, 4)", "5"),
        ("1.5 + 1.5", "3"),
        ("1000000 * 1000000", "1,000,000,000,000"),
    ]);
}

#[test]
fn rejects_non_math() {
    run(&[
        ("1/0", "None"),
        ("foo", "None"),
        ("5", "None"),
        ("firefox", "None"),
        ("2 +", "None"),
        ("", "None"),
        ("e", "None"),
        ("12 apples", "None"),
        ("5 ft", "None"),
        ("20%", "None"),
        ("$30", "None"),
        ("5 ft to kg", "None"),
        ("5 to ft", "None"),
        ("5 apples to ft", "None"),
        ("chrome", "None"),
        ("sqrt(-1)", "None"),
        ("time machine", "None"),
        ("time in mars", "None"),
        ("(2+3", "None"),
        ("100 usd to chf", "None"),
    ]);
}

#[test]
fn percentages() {
    run(&[
        ("$30 + 20%", "$36.00"),
        ("30 + 20%", "36"),
        ("100 - 15%", "85"),
        ("20% of 50", "10"),
        ("20% off 50", "40"),
        ("20% on 50", "60"),
        ("50 as a % of 200", "25%"),
        ("50 is what percent of 200", "25%"),
        ("25 is 20% of what", "125"),
        ("20% of what is 30", "150"),
        ("200 * 15%", "30"),
        ("100 + 10% + 10%", "121"),
        ("15% of 80 + 3", "15"),
        ("8% of $250", "$20.00"),
        ("5 + 5%", "5.25"),
        ("(100 + 20%) * 2", "240"),
        ("30% off $80", "$56.00"),
        ("what is 15% of 200", "30"),
        ("12 as a % of 48", "25%"),
        ("1 as a percent of 3", "33.33333333%"),
        ("50% of 3 km", "1.5 km"),
        ("0.25 to %", "25%"),
    ]);
}

#[test]
fn units() {
    run(&[
        ("5 ft to cm", "152.4 cm"),
        ("1 mi to km", "1.609344 km"),
        ("10 km to mi", "6.213711922 mi"),
        ("100 cm to m", "1 m"),
        ("1 inch to mm", "25.4 mm"),
        ("12 in to ft", "1 ft"),
        ("1 yd to ft", "3 ft"),
        ("2.5 m to ft", "8.202099738 ft"),
        ("1 kg to lb", "2.204622622 lb"),
        ("150 lb to kg", "68.0388555 kg"),
        ("16 oz to lb", "1 lb"),
        ("1 t to kg", "1,000 kg"),
        ("1 stone to lb", "14 lb"),
        ("100 c to f", "212 °F"),
        ("0 c to k", "273.15 K"),
        ("32 f to c", "0 °C"),
        ("-40 c to f", "-40 °F"),
        ("72 f to c", "22.22222222 °C"),
        ("60 mph * 2 h", "120 mi"),
        ("60 mph to kph", "96.56064 km/h"),
        ("5 m/s to km/h", "18 km/h"),
        ("16px to rem", "1 rem"),
        ("24 px to pt", "18 pt"),
        ("1 em to px", "16 px"),
        ("5 ft 3 in to cm", "160.02 cm"),
        ("1 h 30 min + 45 min", "2 h 15 min"),
        ("90 min to h", "1.5 h"),
        ("2 h to min", "120 min"),
        ("1 day to hours", "24 h"),
        ("1 week to days", "7 d"),
        ("3600 s to h", "1 h"),
        ("1 year to days", "365.25 d"),
        ("5 ft 10 in + 3 in", "6 ft 1 in"),
        ("170 cm to ft in", "5 ft 6.929133858 in"),
        ("2 hours 30 minutes to minutes", "150 min"),
        ("1 gb to mb", "1,000 mb"),
        ("1 gib to mib", "1,024 mib"),
        ("8 bits to bytes", "1 byte"),
        ("1 tb to gb", "1,000 gb"),
        ("500 mb to gb", "0.5 gb"),
        ("1 gal to l", "3.785411784 l"),
        ("1 l to ml", "1,000 ml"),
        ("2 cups to ml", "473.176473 ml"),
        ("1 tbsp to tsp", "3 tsp"),
        ("1 fl oz to ml", "29.57352956 ml"),
        ("1 acre to m2", "4,046.856422 m²"),
        ("1 ha to acre", "2.471053815 acre"),
        ("100 sqft to m2", "9.290304 m²"),
        ("1 atm to psi", "14.69594878 psi"),
        ("1 bar to pa", "100,000 pa"),
        ("1 kwh to j", "3,600,000 j"),
        ("1 hp to w", "745.6998716 w"),
        ("1 kcal to kj", "4.184 kj"),
        ("180 deg to rad", "3.141592654 rad"),
        ("1 turn to deg", "360 deg"),
        ("1 mhz to khz", "1,000 khz"),
        ("1 oz to g", "28.34952313 g"),
        ("5 km + 300 m", "5.3 km"),
        ("1 km - 200 m", "0.8 km"),
        ("3 ft + 2 ft", "5 ft"),
        ("2 * 3 ft", "6 ft"),
        ("3 ft * 2", "6 ft"),
        ("10 m / 2", "5 m"),
        ("10 m / 2 s", "5 m/s"),
        ("5 m * 5 m", "25 m²"),
        ("10 km / 2 h", "5 km/h"),
        ("1 ft / 1 m", "0.3048"),
        ("abs(-5 ft)", "5 ft"),
        ("round(5.5 ft)", "6 ft"),
        ("-5 ft + 2 ft", "-3 ft"),
        ("5'3\" to cm", "160.02 cm"),
        ("100 meters to feet", "328.0839895 ft"),
        ("3 workdays to hours", "24 h"),
        ("1 fortnight to days", "14 d"),
        ("1 nmi to km", "1.852 km"),
        ("10 knots to km/h", "18.52 km/h"),
        ("1 lightyear to km", "9,460,730,472,581 km"),
    ]);
}

#[test]
fn bases() {
    run(&[
        ("255 to hex", "0xFF"),
        ("255 to bin", "0b11111111"),
        ("255 to oct", "0o377"),
        ("0xff to dec", "255"),
        ("0b101 + 1", "6"),
        ("255 in binary", "0b11111111"),
        ("1024 to hex", "0x400"),
        ("0xFF to bin", "0b11111111"),
        ("16 to hex", "0x10"),
        ("2.5 to hex", "None"),
    ]);
}

#[test]
fn currencies() {
    run(&[
        ("100 usd to eur", "€80.00"),
        ("100 eur to usd", "$125.00"),
        ("50 gbp to usd", "$66.67"),
        ("1000 jpy to usd", "$6.67"),
        ("100 usd to jpy", "¥15,000.00"),
        ("$100 to eur", "€80.00"),
        ("€50 + $10", "€58.00"),
        ("20 usd to cad", "25.00 CAD"),
        ("$5 * 3", "$15.00"),
        ("$1,200 / 12", "$100.00"),
        ("$19.99 * 3", "$59.97"),
        ("10 usd + 5 usd", "$15.00"),
        ("$100 - 25%", "$75.00"),
        ("$250 * 8%", "$20.00"),
        ("1 usd to eur", "€0.80"),
        ("100 dollars to euros", "€80.00"),
    ]);
}

#[test]
fn dates() {
    run(&[
        ("now", "Sat, Oct 24 2026"),
        ("tomorrow", "Sun, Oct 25 2026"),
        ("yesterday", "Fri, Oct 23 2026"),
        ("3 days from now", "Tue, Oct 27 2026"),
        ("in 2 weeks", "Sat, Nov 7 2026"),
        ("5 days ago", "Mon, Oct 19 2026"),
        ("1 month from now", "Tue, Nov 24 2026"),
        ("1 year from now", "Sun, Oct 24 2027"),
        ("3 days from tomorrow", "Wed, Oct 28 2026"),
        ("10 days after dec 25", "Mon, Jan 4 2027"),
        ("2 days before christmas", "Wed, Dec 23 2026"),
        ("days until christmas", "62 days"),
        ("days until dec 25 2026", "62 days"),
        ("how many days until christmas", "62 days"),
        ("weeks until christmas", "8.86 weeks"),
        ("days since jan 1 2026", "296 days"),
        ("days until 2027-01-01", "69 days"),
        ("days until new year", "69 days"),
        ("days until halloween", "7 days"),
        ("days until tomorrow", "1 day"),
        ("days until thanksgiving", "33 days"),
        ("days until easter", "155 days"),
        ("days between jan 1 2026 and dec 31 2026", "364 days"),
        ("workdays until dec 25 2026", "45 work days"),
        ("5 workdays from now", "Fri, Oct 30 2026"),
        ("3 business days from now", "Wed, Oct 28 2026"),
        ("next friday", "Fri, Oct 30 2026"),
        ("last monday", "Mon, Oct 19 2026"),
        ("next monday", "Mon, Oct 26 2026"),
        ("today + 3 weeks", "Sat, Nov 14 2026"),
        ("dec 25 2026 - 1 week", "Fri, Dec 18 2026"),
        ("days until 12/25/2026", "62 days"),
        ("days until 25 dec", "62 days"),
        ("days until december 25th", "62 days"),
    ]);
}

#[test]
fn unix_time() {
    let n = now().to_string();
    run(&[
        ("unix 1700000000", "2023-11-14 22:13:20 UTC"),
        ("unix 1700000000000", "2023-11-14 22:13:20 UTC"),
        ("unix 0", "1970-01-01 00:00:00 UTC"),
        ("1700000000 to date", "2023-11-14 22:13:20 UTC"),
        ("2023-11-14 to unix", "1699934400"),
    ]);
    run(&[("unix now", &n), ("now to unix", &n), ("unix", &n), ("epoch", &n)]);
    assert_eq!(calc("unix 1700000000").unwrap().caption_answer, "Local 2023-11-14 18:13:20");
}

#[test]
fn time_zones() {
    run(&[
        ("time in tokyo", "9:00 PM"),
        ("time in london", "1:00 PM"),
        ("time in new york", "8:00 AM"),
        ("time in los angeles", "5:00 AM"),
        ("time in sydney", "11:00 PM"),
        ("time in utc", "12:00 PM"),
        ("time in india", "5:30 PM"),
        ("time in auckland", "1:00 AM"),
        ("time in pst", "5:00 AM"),
        ("time in utc+5:30", "5:30 PM"),
        ("tokyo time", "9:00 PM"),
        ("what time is it in paris", "2:00 PM"),
        ("time in dubai", "4:00 PM"),
        ("3pm london to nyc", "10:00 AM"),
        ("9am pst to est", "12:00 PM"),
        ("noon utc to tokyo", "9:00 PM"),
        ("15:00 utc in tokyo", "12:00 AM"),
        ("3pm to tokyo", "4:00 AM"),
        ("10:30 am tokyo to london", "2:30 AM"),
        ("midnight utc to pst", "5:00 PM"),
        ("6pm in london to tokyo", "2:00 AM"),
    ]);
    let r = calc("15:00 utc in tokyo").unwrap();
    assert!(r.caption_answer.contains("next day"), "{r:?}");
}

#[test]
fn card_fields() {
    let r = calc("5 ft to cm").unwrap();
    assert_eq!(r.question, "5 ft to cm");
    assert_eq!(r.answer, "152.4 cm");
    assert_eq!(r.caption_question, "Feet");
    assert_eq!(r.caption_answer, "Centimeters");
    let r = calc("2 + 2").unwrap();
    assert_eq!((r.caption_question.as_str(), r.caption_answer.as_str()), ("Expression", "Result"));
    let r = calc("255 to hex").unwrap();
    assert_eq!(r.caption_answer, "Hexadecimal");
}

#[test]
fn legacy_evaluate() {
    assert_eq!(Calculator::evaluate("2+2"), Some(("4".into(), "= 2+2".into())));
    assert_eq!(Calculator::evaluate("hello"), None);
}

#[test]
fn unit_table_size() {
    assert!(units::name_count() >= 250, "names: {}", units::name_count());
}

#[test]
fn currency_cache_loads_and_rebases() {
    let dir = std::env::temp_dir().join(format!("pcalc-cur-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("currencies.json");
    std::fs::write(&p, r#"{"base":"EUR","date":"2026-10-23","fetched":5,"rates":{"USD":1.25,"GBP":0.5}}"#).unwrap();
    let t = CurrencyTable::load_cache(&p).unwrap();
    let ctx = Ctx { now: now(), local: LocalOff::Fixed(0), cur: &t };
    assert_eq!(calculate_with(&ctx, "1 eur to usd").unwrap().answer, "$1.25");
    assert_eq!(calculate_with(&ctx, "1 gbp to eur").unwrap().answer, "€2.00");
    assert_eq!(t.fetched, 5);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn history_store() {
    let dir = std::env::temp_dir().join(format!("pcalc-hist-{}", std::process::id()));
    let path = dir.join("db/calculator.json");
    let mut h = CalcHistory::open_at(path.clone());
    let a = calc("2+2").unwrap();
    let b = calc("5 ft to cm").unwrap();
    h.add(&a);
    let idb = h.add(&b);
    assert_eq!(h.search("").len(), 2);
    assert_eq!(h.search("")[0].id, idb); // newest first
    assert_eq!(h.search("FT").len(), 1);
    assert_eq!(h.search("152").len(), 1);
    assert!(path.exists() && !path.with_extension("json.tmp").exists());
    assert_eq!(CalcHistory::open_at(path.clone()).len(), 2);
    h.add(&a); // duplicates replace
    assert_eq!(h.len(), 2);
    assert!(h.delete(idb));
    assert!(!h.delete(idb));
    assert_eq!(h.len(), 1);
    h.delete_all();
    assert_eq!(CalcHistory::open_at(path).len(), 0);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn history_caps() {
    let dir = std::env::temp_dir().join(format!("pcalc-cap-{}", std::process::id()));
    let mut h = CalcHistory::open_at(dir.join("c.json"));
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let old = CalcResult { question: "old".into(), answer: "1".into(), caption_question: "".into(), caption_answer: "".into() };
    h.add_at(&old, ts - 91 * 86400);
    let mut r = old.clone();
    for i in 0..(history::MAX_ROWS + 10) {
        r.question = format!("q{i}");
        h.push(&r, ts);
    }
    assert_eq!(h.len(), history::MAX_ROWS);
    assert!(h.search("old").is_empty());
    assert_eq!(h.search("q4105").len(), 1);
    std::fs::remove_dir_all(dir).ok();
}

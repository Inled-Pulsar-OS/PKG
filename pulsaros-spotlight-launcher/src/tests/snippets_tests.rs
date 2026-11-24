use super::*;
use crate::core::store::temp_store;

// 2024-03-05 14:07:09 UTC (a Tuesday), day-of-year 065 (leap year)
const T: i64 = 1_709_647_629;

fn env() -> Env {
    Env { clipboard: "CLIP".into(), now: T, utc_offset: 0, uuid: Box::new(|| "uuid-1".into()), args: HashMap::new() }
}
fn r(t: &str) -> String {
    render(t, &env()).text
}

#[test]
fn plain_text_untouched() {
    assert_eq!(r("hello world"), "hello world");
    assert_eq!(r(""), "");
    assert_eq!(r("ünï ✓ {"), "ünï ✓ {");
}

#[test]
fn simple_placeholders() {
    assert_eq!(r("a {clipboard} b"), "a CLIP b");
    assert_eq!(r("{uuid}"), "uuid-1");
    assert_eq!(r("{date}"), "2024-03-05");
    assert_eq!(r("{time}"), "14:07:09");
    assert_eq!(r("{datetime}"), "2024-03-05 14:07:09");
    assert_eq!(r("{clipboard}{clipboard}"), "CLIPCLIP");
}

#[test]
fn date_formats() {
    assert_eq!(r("{date:%d/%m/%y}"), "05/03/24");
    assert_eq!(r("{date:%A, %B %e %Y}"), "Tuesday, March  5 2024");
    assert_eq!(r("{date:%a %b %j %I:%M %p}"), "Tue Mar 065 02:07 PM");
    assert_eq!(r("{date:%F %T %% %z}"), "2024-03-05 14:07:09 % +0000");
    assert_eq!(r("{date:%Q}"), "%Q");
    assert_eq!(r("{date:100%}"), "100%");
}

#[test]
fn utc_offset_and_day_rollover() {
    let mut e = env();
    e.utc_offset = 10 * 3600;
    assert_eq!(render("{datetime} {date:%z}", &e).text, "2024-03-06 00:07:09 +1000");
    e.utc_offset = -(5 * 3600 + 1800);
    assert_eq!(render("{time} {date:%z}", &e).text, "08:37:09 -0530");
}

#[test]
fn epoch_and_pre_epoch() {
    assert_eq!(format_time("%F %T %a", 0, 0), "1970-01-01 00:00:00 Thu");
    assert_eq!(format_time("%F %T", -1, 0), "1969-12-31 23:59:59");
    assert_eq!(format_time("%F %j", 951_782_400, 0), "2000-02-29 060");
}

#[test]
fn unknown_kept_literally() {
    assert_eq!(r("{foo}"), "{foo}");
    assert_eq!(r("{ clipboard }"), "{ clipboard }");
    assert_eq!(r("{clipboard:x}"), "{clipboard:x}");
    assert_eq!(r("{date:}"), "{date:}");
    assert_eq!(r("{}"), "{}");
    assert_eq!(r("fn() { return {clipboard}; }"), "fn() { return CLIP; }");
}

#[test]
fn unbalanced_braces() {
    assert_eq!(r("{clipboard"), "{clipboard");
    assert_eq!(r("{{clipboard}"), "{CLIP");
    assert_eq!(r("clipboard}"), "clipboard}");
    assert_eq!(r("{a{uuid}"), "{auuid-1");
}

#[test]
fn cursor_offset_in_chars() {
    let x = render("héllo {cursor}world", &env());
    assert_eq!(x.text, "héllo world");
    assert_eq!(x.cursor_offset, Some(6));
    let x = render("{clipboard}{cursor}!", &env());
    assert_eq!(x.cursor_offset, Some(4));
    let x = render("a{cursor}b{cursor}c", &env());
    assert_eq!((x.text.as_str(), x.cursor_offset), ("abc", Some(1)));
    assert_eq!(render("none", &env()).cursor_offset, None);
}

#[test]
fn arguments() {
    let t = r#"Hi {argument name="who" default="you"}, {argument name="who"} / {argument name="x"}"#;
    let x = render(t, &env());
    assert_eq!(x.text, "Hi you, you / ");
    assert_eq!(
        x.arguments,
        vec![
            ArgSpec { name: "who".into(), default: Some("you".into()) },
            ArgSpec { name: "x".into(), default: None }
        ]
    );
    let mut e = env();
    e.args.insert("who".into(), "Bob".into());
    assert_eq!(render(t, &e).text, "Hi Bob, Bob / ");
}

#[test]
fn argument_edge_cases() {
    let x = render(r#"{argument name="a}b" default="}"}"#, &env());
    assert_eq!(x.text, "}");
    assert_eq!(x.arguments[0].name, "a}b");
    assert_eq!(r("{argument}"), "{argument}");
    assert_eq!(r("{argument name=x}"), "{argument name=x}");
    assert_eq!(r(r#"{argument name=""}"#), r#"{argument name=""}"#);
    assert_eq!(r(r#"{argument default="d"}"#), r#"{argument default="d"}"#);
}

#[test]
fn placeholder_output_is_not_reexpanded() {
    let mut e = env();
    e.clipboard = "{uuid}".into();
    assert_eq!(render("{clipboard}", &e).text, "{uuid}");
}

#[test]
fn system_uuid_shape() {
    let u = uuid_v4();
    assert_eq!(u.len(), 36);
    assert_eq!(&u[14..15], "4");
    assert_ne!(u, uuid_v4());
}

#[test]
fn crud_and_store_roundtrip() {
    let st = temp_store("snip");
    let mut s = Snippets::load(&st);
    assert!(s.items.is_empty());
    let a = s.add("Greeting", ";hi", "Hello!").unwrap();
    let b = s.add("Sig", ";sig", "Regards").unwrap();
    assert_eq!((a.as_str(), b.as_str()), ("s1", "s2"));
    assert!(s.add("Dup", ";HI", "x").is_err());
    assert!(s.add("", ";x", "x").is_err());
    assert!(s.add("n", ";x", "").is_err());
    s.save(&st).unwrap();
    let mut s2 = Snippets::load(&st);
    assert_eq!(s2.items, s.items);
    assert!(s2.remove("s1"));
    assert!(!s2.remove("s1"));
    assert_eq!(s2.add("New", "", "t").unwrap(), "s3");
}

#[test]
fn search_ranking() {
    let mut s = Snippets::default();
    s.add("Email signature", ";sig", "x").unwrap();
    s.add("Signal boost", "", "x").unwrap();
    s.add("Address", ";addr", "x").unwrap();
    let names = |q: &str| s.search(q).iter().map(|x| x.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(""), vec!["Address", "Email signature", "Signal boost"]);
    assert_eq!(names(";sig")[0], "Email signature");
    assert_eq!(names("sig"), vec!["Signal boost", "Email signature"]);
    assert!(names("zzz").is_empty());
}

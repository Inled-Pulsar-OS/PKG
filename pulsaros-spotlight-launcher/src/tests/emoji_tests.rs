use super::*;

#[test]
fn dataset_is_big_and_clean() {
    assert!(all().len() >= 1500, "{}", all().len());
    assert!(all().iter().all(|e| !e.name.is_empty() && !e.ch.is_empty()));
}

#[test]
fn no_skin_tone_rows_but_variants_folded() {
    assert!(all().iter().all(|e| !e.name.contains("skin tone")));
    let w = all().iter().find(|e| e.name == "waving hand").unwrap();
    assert_eq!(w.variants.len(), 5);
}

#[test]
fn heart_finds_hearts() {
    let r = search("heart", &Frecency::default(), 0);
    assert!(r.len() > 10);
    assert!(r.iter().take(10).all(|e| e.name.contains("heart")));
    assert!(r.iter().any(|e| e.name == "red heart"));
}

#[test]
fn exact_name_first() {
    assert_eq!(search("red heart", &Frecency::default(), 0)[0].name, "red heart");
}

#[test]
fn keyword_and_no_match() {
    let r = search("rocket", &Frecency::default(), 0);
    assert_eq!(r[0].ch, "🚀");
    assert!(search("zzzzqqq", &Frecency::default(), 0).is_empty());
}

#[test]
fn empty_query_lists_everything_recents_first() {
    let mut fr = Frecency::default();
    assert_eq!(search("", &fr, 0).len(), all().len());
    record_use(&mut fr, "🚀", "", 100);
    record_use(&mut fr, "🎉", "", 200);
    let r = search("", &fr, 300);
    assert_eq!(r[0].ch, "🎉");
    assert_eq!(r[1].ch, "🚀");
    assert_eq!(r.len(), all().len());
}

#[test]
fn variant_use_counts_for_base() {
    let mut fr = Frecency::default();
    record_use(&mut fr, "👋🏽", "", 10);
    assert_eq!(search("", &fr, 10)[0].ch, "👋");
}

#[test]
fn recent_boosts_within_query() {
    let mut fr = Frecency::default();
    let base = search("heart", &fr, 0);
    let pick = base[7].ch.clone();
    record_use(&mut fr, &pick, "heart", 1000);
    record_use(&mut fr, &pick, "heart", 1001);
    let r = search("heart", &fr, 1002);
    let pos = r.iter().position(|e| e.ch == pick).unwrap();
    assert!(pos < 7, "{pos}");
}

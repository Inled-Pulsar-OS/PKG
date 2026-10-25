//! Text ranker. Tiers (higher wins), applied to title, keywords and alias:
//! alias exact 2000 / prefix 700; title exact 1000, prefix 800, word-boundary 650,
//! initials 550 (prefix of initials 500), substring 400, subsequence 50..200;
//! keywords score 60% of the same tiers. A shorter title breaks near-ties.
//! Order of equal scores is made stable by `rank` (title, then id).

#[derive(Clone, Debug, Default)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub keywords: Vec<String>,
    pub alias: Option<String>,
}

fn match_tier(q: &str, text: &str) -> i64 {
    if q.is_empty() || text.is_empty() {
        return 0;
    }
    if text == q {
        return 1000;
    }
    if text.starts_with(q) {
        return 800;
    }
    let chars: Vec<char> = text.chars().collect();
    let qc: Vec<char> = q.chars().collect();
    for i in 1..chars.len() {
        if !chars[i - 1].is_alphanumeric() && chars[i..].starts_with(&qc) {
            return 650;
        }
    }
    if qc.len() >= 2 {
        let initials: Vec<char> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter_map(|w| w.chars().next())
            .collect();
        if initials == qc {
            return 550;
        }
        if initials.starts_with(&qc) {
            return 500;
        }
    }
    if text.contains(q) {
        return 400;
    }
    // subsequence, tighter span is better
    let mut qi = 0;
    let mut first = None;
    let mut last = 0;
    for (i, &c) in chars.iter().enumerate() {
        if qi < qc.len() && c == qc[qi] {
            first.get_or_insert(i);
            last = i;
            qi += 1;
        }
    }
    if qi == qc.len() {
        let span = (last - first.unwrap_or(0) + 1) as i64;
        let gaps = span - qc.len() as i64;
        return (200 - gaps * 10).max(50);
    }
    0
}

/// `None` when the item does not match at all.
pub fn score(query: &str, item: &Item) -> Option<i64> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return None;
    }
    let mut best = 0i64;
    if let Some(a) = &item.alias {
        let a = a.to_lowercase();
        if a == q {
            return Some(20000);
        }
        if a.starts_with(&q) {
            best = 700;
        }
    }
    best = best.max(match_tier(&q, &item.title.to_lowercase()));
    for k in &item.keywords {
        best = best.max(match_tier(&q, &k.to_lowercase()) * 6 / 10);
    }
    if best == 0 {
        return None;
    }
    let len_penalty = (item.title.chars().count() as i64).min(40) / 4;
    Some(best * 10 - len_penalty)
}

/// Rank items; stable order: score desc, title, id.
pub fn rank<'a>(query: &str, items: &'a [Item]) -> Vec<(&'a Item, i64)> {
    let mut v: Vec<_> = items.iter().filter_map(|i| score(query, i).map(|s| (i, s))).collect();
    v.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.title.to_lowercase().cmp(&b.0.title.to_lowercase()))
            .then_with(|| a.0.id.cmp(&b.0.id))
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn it(id: &str, title: &str, kw: &[&str]) -> Item {
        Item { id: id.into(), title: title.into(), keywords: kw.iter().map(|s| s.to_string()).collect(), alias: None }
    }
    fn order(q: &str, items: &[Item]) -> Vec<String> {
        rank(q, items).into_iter().map(|(i, _)| i.id.clone()).collect()
    }

    #[test]
    fn empty_query_no_match() {
        assert_eq!(score("", &it("a", "A", &[])), None);
        assert_eq!(score("  ", &it("a", "A", &[])), None);
    }

    #[test]
    fn no_match() {
        assert_eq!(score("zzz", &it("a", "Firefox", &[])), None);
    }

    #[test]
    fn golden_ff_firefox_over_fast_fonts() {
        // Firefox carries the common "ff" abbreviation as a keyword.
        let items = [it("fonts", "Fast Fonts", &[]), it("ff", "Firefox", &["ff", "browser"])];
        assert_eq!(order("ff", &items), vec!["ff", "fonts"]);
    }

    #[test]
    fn golden_tiers() {
        let items = [
            it("sub", "Xfirex", &[]),
            it("word", "My Fire Thing", &[]),
            it("prefix", "Fireplace Viewer", &[]),
            it("exact", "Fire", &[]),
            it("kw", "Browser", &["fire"]),
        ];
        assert_eq!(order("fire", &items), vec!["exact", "prefix", "word", "kw", "sub"]);
    }

    #[test]
    fn initials() {
        let items = [it("a", "Visual Studio Code", &[]), it("b", "Vivaldi", &[])];
        assert_eq!(order("vsc", &items), vec!["a"]);
        assert_eq!(order("vs", &items)[0], "a");
    }

    #[test]
    fn initials_beat_subsequence() {
        let items = [it("sub", "Fxsx", &[]), it("ini", "Fast Fonts", &[])];
        assert_eq!(order("ff", &items)[0], "ini");
    }

    #[test]
    fn subsequence_tighter_wins() {
        let items = [it("loose", "Firefox Nightly", &[]), it("tight", "Frox", &[])];
        assert_eq!(order("frox", &items)[0], "tight");
        assert!(score("fx", &items[0]).is_some());
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(score("FIRE", &it("a", "firefox", &[])), score("fire", &it("a", "Firefox", &[])));
    }

    #[test]
    fn alias_exact_is_top() {
        let mut a = it("a", "Zed", &[]);
        a.alias = Some("ff".into());
        let b = it("b", "ff", &[]);
        assert_eq!(order("ff", &[b, a]), vec!["a", "b"]);
    }

    #[test]
    fn alias_prefix_matches() {
        let mut a = it("a", "Zed", &[]);
        a.alias = Some("edit".into());
        assert!(score("ed", &a).is_some());
    }

    #[test]
    fn multiword_query() {
        let items = [it("a", "Dark Style", &[]), it("b", "Darkroom", &[])];
        assert_eq!(order("dark s", &items), vec!["a"]);
    }

    #[test]
    fn shorter_title_wins_tie() {
        let items = [it("long", "Terminal Emulator Pro", &[]), it("short", "Terminal Pro", &[])];
        assert_eq!(order("term", &items)[0], "short");
    }

    #[test]
    fn stable_tie_break_by_title_then_id() {
        let items = [it("2", "Same", &[]), it("1", "Same", &[]), it("0", "Same", &[])];
        assert_eq!(order("same", &items), vec!["0", "1", "2"]);
        let items = [it("b", "Bbb Tool", &[]), it("a", "Aaa Tool", &[])];
        assert_eq!(order("tool", &items), vec!["a", "b"]);
    }

    #[test]
    fn keyword_exact_beats_title_substring() {
        let items = [it("t", "Xdarkx", &[]), it("k", "Appearance", &["dark"])];
        assert_eq!(order("dark", &items)[0], "k");
    }
}

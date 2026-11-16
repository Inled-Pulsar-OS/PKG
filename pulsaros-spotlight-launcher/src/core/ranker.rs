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
#[path = "../tests/ranker_tests.rs"]
mod tests;

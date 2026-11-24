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

/// Lowercased text with its chars and initials computed once.
#[derive(Clone, Debug, Default)]
pub struct Prepared {
    text: String,
    chars: Vec<char>,
    initials: Vec<char>,
}

impl Prepared {
    pub fn new(raw: &str) -> Self {
        let text = raw.to_lowercase();
        let chars = text.chars().collect();
        let initials = text.split(|c: char| !c.is_alphanumeric()).filter_map(|w| w.chars().next()).collect();
        Prepared { text, chars, initials }
    }
}

/// An `Item` with title and keywords prepared once. The alias stays out: it changes with settings.
#[derive(Clone, Debug, Default)]
pub struct Indexed {
    pub id: String,
    /// Lowercased title: the sort tie-break key.
    pub title_lc: String,
    title: Prepared,
    title_len: i64,
    keywords: Vec<Prepared>,
}

impl Indexed {
    pub fn new(id: &str, title: &str, keywords: &[String]) -> Self {
        let title_p = Prepared::new(title);
        Indexed {
            id: id.into(),
            title_lc: title_p.text.clone(),
            title: title_p,
            title_len: title.chars().count() as i64,
            keywords: keywords.iter().map(|k| Prepared::new(k)).collect(),
        }
    }
}

/// A query lowercased and split into chars once, reused for every item.
pub struct Query {
    q: String,
    qc: Vec<char>,
}

impl Query {
    /// `None` for a blank query.
    pub fn new(query: &str) -> Option<Query> {
        let q = query.trim().to_lowercase();
        (!q.is_empty()).then(|| Query { qc: q.chars().collect(), q })
    }
}

fn match_tier(q: &Query, t: &Prepared) -> i64 {
    let (text, chars, qc) = (t.text.as_str(), &t.chars, &q.qc);
    if text.is_empty() {
        return 0;
    }
    if text == q.q {
        return 1000;
    }
    if text.starts_with(&q.q) {
        return 800;
    }
    for i in 1..chars.len() {
        if !chars[i - 1].is_alphanumeric() && chars[i..].starts_with(qc) {
            return 650;
        }
    }
    if qc.len() >= 2 {
        if t.initials == *qc {
            return 550;
        }
        if t.initials.starts_with(qc) {
            return 500;
        }
    }
    if text.contains(&q.q) {
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
pub fn score_indexed(q: &Query, item: &Indexed, alias: Option<&str>) -> Option<i64> {
    let mut best = 0i64;
    if let Some(a) = alias {
        let a = a.to_lowercase();
        if a == q.q {
            return Some(20000);
        }
        if a.starts_with(&q.q) {
            best = 700;
        }
    }
    best = best.max(match_tier(q, &item.title));
    for k in &item.keywords {
        best = best.max(match_tier(q, k) * 6 / 10);
    }
    if best == 0 {
        return None;
    }
    Some(best * 10 - item.title_len.min(40) / 4)
}

/// `None` when the item does not match at all.
pub fn score(query: &str, item: &Item) -> Option<i64> {
    let q = Query::new(query)?;
    score_indexed(&q, &Indexed::new(&item.id, &item.title, &item.keywords), item.alias.as_deref())
}

/// Rank items; stable order: score desc, title, id.
pub fn rank<'a>(query: &str, items: &'a [Item]) -> Vec<(&'a Item, i64)> {
    let Some(q) = Query::new(query) else { return Vec::new() };
    let mut v: Vec<_> = items
        .iter()
        .filter_map(|i| {
            let ix = Indexed::new(&i.id, &i.title, &i.keywords);
            score_indexed(&q, &ix, i.alias.as_deref()).map(|s| (i, s, ix.title_lc))
        })
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)).then_with(|| a.0.id.cmp(&b.0.id)));
    v.into_iter().map(|(i, s, _)| (i, s)).collect()
}

#[cfg(test)]
#[path = "../tests/ranker_tests.rs"]
mod tests;

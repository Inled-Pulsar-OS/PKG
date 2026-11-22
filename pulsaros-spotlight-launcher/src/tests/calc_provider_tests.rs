use super::*;

fn result(q: &str, a: &str) -> CalcResult {
    CalcResult { question: q.into(), answer: a.into(), caption_question: String::new(), caption_answer: String::new() }
}

fn ids(p: &dyn Provider, q: &str) -> Vec<String> {
    p.instant(&Query::new(q)).into_iter().map(|r| r.id).collect()
}

#[test]
fn calculator_answers_an_expression() {
    let rows = CalculatorProvider.instant(&Query::new("2+2"));
    assert_eq!((rows[0].id.as_str(), rows[0].title.as_str(), rows[0].kind), ("calc:4", "4", RowKind::Calc));
    assert_eq!(CalculatorProvider.id(), "calculator");
}

#[test]
fn calculator_ignores_plain_words() {
    assert!(ids(&CalculatorProvider, "banana").is_empty());
}

#[test]
fn calculator_offers_history_command_for_a_prefix() {
    assert_eq!(ids(&CalculatorProvider, "calculator hist"), ["calc:history"]);
    assert!(ids(&CalculatorProvider, "ca").is_empty());
}

#[test]
fn history_lists_matches_newest_first() {
    let path = std::env::temp_dir().join(format!("pulsaros-calcprov-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let h = Rc::new(RefCell::new(CalcHistory::open_at(path.clone())));
    let first = h.borrow_mut().add(&result("1+1", "2"));
    let second = h.borrow_mut().add(&result("2+2", "4"));
    let p = CalcHistoryProvider::new(h);
    assert_eq!(ids(&p, ""), [format!("calchist:{second}"), format!("calchist:{first}")]);
    assert_eq!(ids(&p, "2+2"), [format!("calchist:{second}")]);
    assert_eq!(p.id(), "calc-history");
    let _ = std::fs::remove_file(path);
}

//! Calculator and calculator history as `Provider`s.
use std::cell::RefCell;
use std::rc::Rc;

use super::provider::{Provider, Query};
use super::root_search::{Row, RowKind};
use crate::calculator::{CalcHistory, CalcResult, Calculator, HistoryRow};

const ICON: &str = "accessories-calculator-symbolic";

pub fn calc_row(c: &CalcResult, id: &str) -> Row {
    Row::simple(RowKind::Calc, id, &c.answer, &c.question, ICON)
}

pub fn history_result(h: &HistoryRow) -> CalcResult {
    CalcResult {
        question: h.question.clone(),
        answer: h.answer.clone(),
        caption_question: h.caption_question.clone(),
        caption_answer: h.caption_answer.clone(),
    }
}

/// The answer card of a calculation, plus the "Calculator History" command when the query names it.
pub struct CalculatorProvider;

impl Provider for CalculatorProvider {
    fn instant(&self, query: &Query) -> Vec<Row> {
        let mut rows: Vec<Row> =
            Calculator::calculate(&query.text).map(|c| calc_row(&c, &format!("calc:{}", c.answer))).into_iter().collect();
        let ql = query.text.to_lowercase();
        if ql.len() >= 3 && "calculator history".contains(&ql) {
            rows.push(Row::simple(RowKind::Command, "calc:history", "Calculator History", "Calculator", ICON));
        }
        rows
    }
}

/// Saved calculations matching the query, newest first, as `calchist:<id>` rows.
pub struct CalcHistoryProvider {
    history: Rc<RefCell<CalcHistory>>,
}

impl CalcHistoryProvider {
    pub fn new(history: Rc<RefCell<CalcHistory>>) -> Self {
        CalcHistoryProvider { history }
    }
}

impl Provider for CalcHistoryProvider {
    fn instant(&self, query: &Query) -> Vec<Row> {
        let rows = self.history.borrow().search(&query.text);
        rows.iter().map(|h| calc_row(&history_result(h), &format!("calchist:{}", h.id))).collect()
    }
}

#[cfg(test)]
#[path = "../tests/calc_provider_tests.rs"]
mod tests;

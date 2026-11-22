use std::cell::Cell;
use std::rc::Rc;

use super::*;
use crate::core::root_search::Section;

fn run(calls: Rc<Cell<u32>>) -> Rc<RootRun> {
    Rc::new(RootRun::new(move || {
        calls.set(calls.get() + 1);
        vec![Section {
            title: "Results".into(),
            rows: vec![
                Row::simple(RowKind::App, "app:a", "A", "", ""),
                Row::simple(RowKind::Command, "gnome:c", "C", "", ""),
                Row::simple(RowKind::File, "file:f", "F", "", ""),
            ],
        }]
    }))
}

#[test]
fn keeps_only_app_rows() {
    let p = AppsProvider::new(run(Rc::default()));
    let ids: Vec<String> = p.instant(&Query::new("a")).into_iter().map(|r| r.id).collect();
    assert_eq!(ids, ["app:a"]);
}

#[test]
fn shares_one_run_with_commands() {
    let calls = Rc::new(Cell::new(0));
    let shared = run(calls.clone());
    let apps = AppsProvider::new(shared.clone());
    let cmds = crate::core::commands_provider::CommandsProvider::new(shared);
    let q = Query::new("a");
    apps.instant(&q);
    cmds.instant(&q);
    assert_eq!(calls.get(), 1);
}

use super::*;
use crate::core::root_search::Section;

#[test]
fn keeps_only_command_rows_in_section_order() {
    let run = Rc::new(RootRun::new(|| {
        vec![
            Section { title: "Favorites".into(), rows: vec![Row::simple(RowKind::Command, "gnome:b", "B", "", "")] },
            Section {
                title: "Results".into(),
                rows: vec![
                    Row::simple(RowKind::App, "app:a", "A", "", ""),
                    Row::simple(RowKind::Command, "gnome:c", "C", "", ""),
                ],
            },
        ]
    }));
    let p = CommandsProvider::new(run);
    let ids: Vec<String> = p.instant(&Query::new("x")).into_iter().map(|r| r.id).collect();
    assert_eq!(ids, ["gnome:b", "gnome:c"]);
    assert_eq!(p.id(), "commands");
}

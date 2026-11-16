use super::*;

#[test]
fn row_to_entry_maps_fields_in_order() {
    let row = (
        7u32,
        "Title".to_string(),
        "WmClass".to_string(),
        "app.id".to_string(),
        "workspace-1".to_string(),
    );

    let entry = row_to_entry(row);

    assert_eq!(entry.id, 7);
    assert_eq!(entry.title, "Title");
    assert_eq!(entry.wm_class, "WmClass");
    assert_eq!(entry.app_id, "app.id");
    assert_eq!(entry.workspace, "workspace-1");
}

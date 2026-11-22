use super::*;
use crate::core::root_search::RowKind;

struct Fake {
    name: &'static str,
    ids: Vec<&'static str>,
    job: bool,
}

fn row(id: &str, title: &str) -> Row {
    Row::simple(RowKind::App, id, title, "", "")
}

impl Provider for Fake {
    fn id(&self) -> &str {
        self.name
    }
    fn instant(&self, q: &Query) -> Vec<Row> {
        self.ids.iter().map(|id| row(id, &format!("{}:{}", self.name, q.text))).collect()
    }
    fn background(&self, _q: &Query) -> Option<Job> {
        self.job.then(|| Job { kind: JobKind::Files, title: "T", run: Box::new(|_| vec![row("late", "late")]) })
    }
}

fn registry() -> Registry {
    let mut r = Registry::default();
    r.register(Box::new(Fake { name: "a", ids: vec!["x", "y"], job: false }));
    r.register(Box::new(Fake { name: "b", ids: vec!["y", "z"], job: true }));
    r
}

#[test]
fn query_trims_and_detects_empty() {
    assert!(Query::new("  \t").is_empty());
    let q = Query::new(" hi ");
    assert_eq!((q.raw.as_str(), q.text.as_str()), (" hi ", "hi"));
}

#[test]
fn registry_keeps_registration_order_and_dedups_by_id() {
    let rows = registry().instant(&Query::new("q"));
    let got: Vec<(&str, &str)> = rows.iter().map(|r| (r.id.as_str(), r.title.as_str())).collect();
    assert_eq!(got, [("x", "a:q"), ("y", "a:q"), ("z", "b:q")]);
}

#[test]
fn registry_collects_only_providers_with_a_job() {
    let jobs = registry().background(&Query::new("q"));
    assert_eq!(jobs.len(), 1);
    let job = jobs.into_iter().next().unwrap();
    assert_eq!((job.kind, job.title), (JobKind::Files, "T"));
    let rows = (job.run)(&mut WorkerCtx::default());
    assert_eq!(rows[0].id, "late");
}

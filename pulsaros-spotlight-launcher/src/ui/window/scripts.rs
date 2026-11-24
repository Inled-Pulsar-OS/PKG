//! Script commands: run one off the UI thread and show its outcome.
use super::views::{spawn_poll, View};
use super::*;
use crate::core::script_commands::{self, Outcome, SystemRunner};

impl SpotlightWindow {
    // ---- script commands -------------------------------------------------------------

    pub(super) fn run_script(self: &Rc<Self>, id: &str) {
        let Some(script) = self.x.scripts.borrow().iter().find(|s| format!("script:{}", s.rel) == id).cloned() else {
            self.toast("Script not found", None);
            return;
        };
        let mut values = Vec::new();
        if !script.header.arguments.is_empty() {
            if !self.args_ready(id) {
                return;
            }
            values = self.arg_values(id);
        }
        self.frecency.borrow_mut().record_launch(id, "", now_secs());
        self.save_frecency();
        self.loading_begin();
        let (s, title) = (self.clone(), script.header.title.clone());
        spawn_poll(
            move || script_commands::run(&script, &values, &SystemRunner),
            move |outcome| {
                s.loading_end();
                s.script_outcome(&title, outcome);
            },
        );
    }

    pub(super) fn script_outcome(self: &Rc<Self>, title: &str, outcome: Outcome) {
        let shown = self.window.is_visible();
        match outcome {
            Outcome::Silent => self.window.set_visible(false),
            Outcome::Compact(l) | Outcome::Inline(l) => {
                let msg = if l.is_empty() { format!("{title}: done") } else { l };
                self.toast(&msg, None);
            }
            Outcome::FullOutput(text) => {
                self.x.output.buffer().set_text(&text);
                if !shown {
                    self.window.present();
                }
                self.push_view(View::Output, title);
            }
            Outcome::Failed(m) => {
                let msg = format!("{title} failed: {m}");
                if shown {
                    self.toast(&msg, None);
                } else {
                    let _ = std::process::Command::new("notify-send").arg("Spotlight").arg(&msg).spawn();
                }
            }
        }
    }
}

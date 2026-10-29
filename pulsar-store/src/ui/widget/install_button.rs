//! The Get / progress ring / Open button.
//!
//! One pill that changes in place: Get becomes a ring while installing
//! (tap it to cancel), then Open. Updates show Update.

use crate::core::i18n;
use crate::core::install::Action;
use crate::core::model::{InstallState, Item, SourceKind};
use crate::ui::ctx::{Change, Ctx, Op};
use crate::ui::motion;
use crate::ui::widget::progress_ring::ProgressRing;
use crate::{ptr, tr};
use gtk4 as gtk;
use gtk4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonState {
    Get,
    /// Working; the fraction is `None` when unknown.
    Installing(Option<f32>),
    Open,
    Update,
    /// Installed but not something that can be launched (an extension, skill, or plugin).
    Installed,
}

/// Decides what the button shows from the item's state and any running operation.
pub fn button_state(item: &Item, state: &InstallState, op: Option<&Op>) -> ButtonState {
    if let Some(op) = op {
        return ButtonState::Installing(op.fraction);
    }
    match state {
        InstallState::NotInstalled => ButtonState::Get,
        InstallState::UpdateAvailable { .. } => ButtonState::Update,
        InstallState::Installed { .. } => match item.kind {
            SourceKind::Flatpak | SourceKind::System => ButtonState::Open,
            _ => ButtonState::Installed,
        },
    }
}

#[derive(Clone)]
pub struct InstallButton {
    root: gtk::Stack,
}

impl InstallButton {
    pub fn new(ctx: &Ctx, item: &Item) -> Self {
        Self::build(ctx, item, false)
    }

    /// The large, solid-accent form used in a product header.
    pub fn prominent(ctx: &Ctx, item: &Item) -> Self {
        Self::build(ctx, item, true)
    }

    fn build(ctx: &Ctx, item: &Item, prominent: bool) -> Self {
        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        stack.set_transition_duration(motion::FAST_MS);
        stack.set_valign(gtk::Align::Center);
        stack.set_halign(gtk::Align::End);

        let button = gtk::Button::new();
        button.add_css_class("store-pill");
        button.add_css_class("get");
        if prominent {
            button.add_css_class("prominent");
        }
        stack.add_named(&button, Some("button"));

        let ring = ProgressRing::new(28);
        let ring_button = gtk::Button::new();
        ring_button.add_css_class("flat");
        ring_button.add_css_class("store-ring-button");
        ring_button.set_child(Some(ring.widget()));
        ring_button.set_tooltip_text(Some(&ptr!("button", "Cancel")));
        ring_button.update_property(&[gtk::accessible::Property::Label(&i18n::format(
            &tr!("{action}: {name}"),
            &[
                ("action", &ptr!("button", "Cancel")),
                ("name", &ctx.name(item)),
            ],
        ))]);
        stack.add_named(&ring_button, Some("ring"));

        let id = item.id.clone();
        let this = Self { root: stack };

        // Clicking acts on whatever the current state is.
        {
            let (click_ctx, click_item) = (ctx.clone(), item.clone());
            button.connect_clicked(move |_| {
                let state = click_ctx.state(&click_item.id);
                let op = click_ctx.op(&click_item.id);
                match button_state(&click_item, &state, op.as_ref()) {
                    ButtonState::Get => click_ctx.act(Action::Install(None), click_item.clone()),
                    ButtonState::Update => click_ctx.act(Action::Update, click_item.clone()),
                    ButtonState::Open => click_ctx.open_item(&click_item),
                    _ => {}
                }
            });
            let (cancel_ctx, cancel_id) = (ctx.clone(), id.clone());
            ring_button.connect_clicked(move |_| cancel_ctx.cancel(&cancel_id));
        }

        let refresh = {
            let (ctx, item, stack, button, ring) = (
                ctx.clone(),
                item.clone(),
                this.root.clone(),
                button.clone(),
                ring.clone(),
            );
            move || {
                let state = button_state(&item, &ctx.state(&item.id), ctx.op(&item.id).as_ref());
                match state {
                    ButtonState::Installing(fraction) => {
                        ring.set_fraction(fraction);
                        stack.set_visible_child_name("ring");
                    }
                    other => {
                        let label = match other {
                            ButtonState::Get => ptr!("button", "Get"),
                            ButtonState::Open => ptr!("button", "Open"),
                            ButtonState::Update => ptr!("button", "Update"),
                            _ => ptr!("button", "Installed"),
                        };
                        button.set_label(&label);
                        // Screen readers hear which item the button acts on.
                        button.update_property(&[gtk::accessible::Property::Label(&i18n::format(
                            &tr!("{action}: {name}"),
                            &[("action", &label), ("name", &ctx.name(&item))],
                        ))]);
                        button.set_sensitive(other != ButtonState::Installed);
                        stack.set_visible_child_name("button");
                    }
                }
            }
        };
        refresh();
        let relevant_id = id;
        ctx.subscribe(&this.root, move |c| {
            let mine = match c {
                Change::State(i) | Change::Op(i) => *i == relevant_id,
                Change::States | Change::Catalog => true,
                Change::Icons => false,
            };
            if mine {
                refresh();
            }
        });
        this
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ports::CancelToken;

    fn item(kind: &str) -> Item {
        serde_json::from_str(&format!(r#"{{"id":"x","type":"{kind}"}}"#)).unwrap()
    }

    fn op(f: Option<f32>) -> Op {
        Op {
            action: Action::Install(None),
            fraction: f,
            cancel: CancelToken::new(),
            queued: false,
        }
    }

    #[test]
    fn state_machine() {
        let app = item("flatpak");
        let ext = item("gnome_extension");
        let installed = InstallState::Installed { version: None };
        assert_eq!(
            button_state(&app, &InstallState::NotInstalled, None),
            ButtonState::Get
        );
        assert_eq!(button_state(&app, &installed, None), ButtonState::Open);
        assert_eq!(button_state(&ext, &installed, None), ButtonState::Installed);
        let upd = InstallState::UpdateAvailable {
            installed: "1".into(),
            available: "2".into(),
        };
        assert_eq!(button_state(&app, &upd, None), ButtonState::Update);
    }

    #[test]
    fn running_operation_wins() {
        let app = item("flatpak");
        let o = op(Some(0.4));
        assert_eq!(
            button_state(&app, &InstallState::NotInstalled, Some(&o)),
            ButtonState::Installing(Some(0.4))
        );
        assert_eq!(
            button_state(
                &app,
                &InstallState::Installed { version: None },
                Some(&op(None))
            ),
            ButtonState::Installing(None)
        );
    }
}

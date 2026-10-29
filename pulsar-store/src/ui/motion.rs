//! Motion tokens and helpers. No animation runs longer than `MAX_MS`.

use adw::prelude::*;
use gtk4 as gtk;

pub const FAST_MS: u32 = 150;
pub const BASE_MS: u32 = 250;
pub const SLOW_MS: u32 = 350;
pub const MAX_MS: u32 = 400;

/// Spring parameters shared by springy motion (damping ratio, mass, stiffness).
pub const SPRING_DAMPING: f64 = 0.85;
pub const SPRING_MASS: f64 = 1.0;
pub const SPRING_STIFFNESS: f64 = 380.0;

/// Clamps a requested duration to the motion budget.
pub fn clamp_ms(ms: u32) -> u32 {
    ms.min(MAX_MS)
}

/// True unless the user turned animations off (`gtk-enable-animations`).
pub fn animations_enabled() -> bool {
    gtk::Settings::default().map_or(true, |s| s.is_gtk_enable_animations())
}

/// Animates `from` to `to` over `ms` and calls `apply` on every frame.
/// With animations off, applies `to` once, immediately.
pub fn animate(
    widget: &impl IsA<gtk::Widget>,
    from: f64,
    to: f64,
    ms: u32,
    apply: impl Fn(f64) + 'static,
) {
    if !animations_enabled() {
        apply(to);
        return;
    }
    let target = adw::CallbackAnimationTarget::new(apply);
    let anim = adw::TimedAnimation::new(widget, from, to, clamp_ms(ms), target);
    anim.set_easing(adw::Easing::EaseOutCubic);
    anim.play();
}

/// Fades a widget in. With animations off it is simply shown.
pub fn fade_in(widget: &(impl IsA<gtk::Widget> + Clone + 'static), ms: u32) {
    if !animations_enabled() {
        return;
    }
    widget.set_opacity(0.0);
    let target = widget.clone();
    animate(widget, 0.0, 1.0, ms, move |v| target.set_opacity(v));
}

/// Like `animate`, but with spring physics, which suits direct manipulation.
pub fn animate_spring(
    widget: &impl IsA<gtk::Widget>,
    from: f64,
    to: f64,
    apply: impl Fn(f64) + 'static,
) {
    if !animations_enabled() {
        apply(to);
        return;
    }
    let target = adw::CallbackAnimationTarget::new(apply);
    let spring = adw::SpringParams::new(SPRING_DAMPING, SPRING_MASS, SPRING_STIFFNESS);
    adw::SpringAnimation::new(widget, from, to, spring, target).play();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_stay_within_budget() {
        for ms in [FAST_MS, BASE_MS, SLOW_MS] {
            assert!(ms <= MAX_MS);
        }
        assert_eq!(clamp_ms(900), MAX_MS);
        assert_eq!(clamp_ms(100), 100);
    }
}

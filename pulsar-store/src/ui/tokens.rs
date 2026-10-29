//! Design tokens. Colors live in `assets/css/{light,dark}.css` as `@store_*`
//! names; numeric tokens live here. A test keeps the two in sync.

/// Spacing scale, in pixels.
pub const SPACE_XS: i32 = 4;
pub const SPACE_S: i32 = 8;
pub const SPACE_M: i32 = 12;
pub const SPACE_L: i32 = 16;
pub const SPACE_XL: i32 = 24;
pub const SPACE_XXL: i32 = 32;

/// Corner radii, in pixels. Inner radii are derived so they stay concentric
/// with the surface that contains them.
pub const RADIUS_WINDOW: f32 = 18.0;
pub const RADIUS_CARD: f32 = 18.0;
pub const RADIUS_TILE: f32 = 14.0;
pub const RADIUS_ICON: f32 = 14.0;
pub const RADIUS_PILL: f32 = 999.0;

/// Gap between a surface's edge and the control nested inside it.
pub const INSET_TOOLBAR: f32 = 8.0;

/// Radius for a shape nested `inset` pixels inside a shape of radius `outer`.
pub fn concentric(outer: f32, inset: f32) -> f32 {
    (outer - inset).max(0.0)
}

/// Blur radius of the glass surface, in pixels.
pub const GLASS_BLUR: f64 = 24.0;

/// Color names every theme file must define.
pub const COLOR_TOKENS: [&str; 11] = [
    "store_surface",
    "store_surface_hover",
    "store_hairline",
    "store_text_dim",
    "store_surface_flat",
    "store_glass_tint",
    "store_glass_edge",
    "store_glass_edge_dim",
    "store_glass_shadow",
    "store_hero_scrim",
    "store_accent_text",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn css(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/css")
            .join(name);
        std::fs::read_to_string(path).unwrap()
    }

    fn defined(css: &str) -> BTreeSet<String> {
        css.lines()
            .filter_map(|l| l.strip_prefix("@define-color "))
            .filter_map(|l| l.split_whitespace().next())
            .map(str::to_string)
            .collect()
    }

    fn used(css: &str) -> BTreeSet<String> {
        css.split('@')
            .skip(1)
            .map(|s| {
                s.chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .filter(|n| n.starts_with("store_"))
            .collect()
    }

    #[test]
    fn both_themes_define_every_token() {
        let want: BTreeSet<String> = COLOR_TOKENS.iter().map(|s| s.to_string()).collect();
        assert_eq!(defined(&css("light.css")), want);
        assert_eq!(defined(&css("dark.css")), want);
    }

    #[test]
    fn components_only_use_defined_tokens() {
        let want: BTreeSet<String> = COLOR_TOKENS.iter().map(|s| s.to_string()).collect();
        for file in ["base.css", "glass.css"] {
            let extra: Vec<_> = used(&css(file)).difference(&want).cloned().collect();
            assert!(extra.is_empty(), "{file} uses undefined tokens: {extra:?}");
        }
    }

    #[test]
    fn concentric_radius() {
        assert_eq!(concentric(18.0, 8.0), 10.0);
        assert_eq!(concentric(4.0, 8.0), 0.0);
    }
}

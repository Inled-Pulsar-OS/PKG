//! Where an installed item puts its files, for the "Package files" section.

use crate::core::model::{Item, SourceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    SkillPrompt,
    Manifest,
    PluginFiles,
    ExtensionMetadata,
    ExtensionCode,
    ExtensionStyle,
    ExtensionSchemas,
    AppFiles,
    DesktopEntry,
    Binary,
}

impl FileRole {
    pub fn icon(self) -> &'static str {
        match self {
            Self::SkillPrompt => "text-x-generic-symbolic",
            Self::Manifest | Self::ExtensionMetadata => "document-properties-symbolic",
            Self::PluginFiles | Self::ExtensionSchemas => "folder-symbolic",
            Self::ExtensionCode | Self::Binary => "application-x-executable-symbolic",
            Self::ExtensionStyle => "image-x-generic-symbolic",
            Self::AppFiles => "package-x-generic-symbolic",
            Self::DesktopEntry => "preferences-desktop-display-symbolic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageFile {
    pub path: String,
    pub role: FileRole,
}

fn file(path: String, role: FileRole) -> PackageFile {
    PackageFile { path, role }
}

/// Paths match where the store's sources actually install (user scope for
/// Flatpak, extensions, and Sayri; system paths for native packages).
pub fn package_files(item: &Item) -> Vec<PackageFile> {
    let id = &item.id;
    match item.kind {
        SourceKind::SayriSkill => vec![file(
            format!("~/.config/sayri/skills/{id}/SKILL.md"),
            FileRole::SkillPrompt,
        )],
        SourceKind::SayriPlugin => vec![
            file(
                format!("~/.config/sayri/plugins/{id}/manifest.json"),
                FileRole::Manifest,
            ),
            file(
                format!("~/.config/sayri/plugins/{id}/"),
                FileRole::PluginFiles,
            ),
        ],
        SourceKind::GnomeExtension => {
            let uuid = item.extension_uuid();
            let base = format!("~/.local/share/gnome-shell/extensions/{uuid}");
            vec![
                file(format!("{base}/metadata.json"), FileRole::ExtensionMetadata),
                file(format!("{base}/extension.js"), FileRole::ExtensionCode),
                file(format!("{base}/stylesheet.css"), FileRole::ExtensionStyle),
                file(format!("{base}/schemas/"), FileRole::ExtensionSchemas),
            ]
        }
        SourceKind::Flatpak => vec![
            file(
                format!("~/.local/share/flatpak/app/{id}/"),
                FileRole::AppFiles,
            ),
            file(
                format!("~/.local/share/flatpak/exports/share/applications/{id}.desktop"),
                FileRole::DesktopEntry,
            ),
        ],
        SourceKind::System | SourceKind::Unknown => vec![
            file(format!("/usr/bin/{}", item.native_name()), FileRole::Binary),
            file(
                format!("/usr/share/applications/{}.desktop", item.native_name()),
                FileRole::DesktopEntry,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(json: &str) -> Item {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn paths_per_kind() {
        let skill = package_files(&item(r#"{"id":"s","type":"sayri_skill"}"#));
        assert_eq!(skill[0].path, "~/.config/sayri/skills/s/SKILL.md");
        let ext = package_files(&item(
            r#"{"id":"e","type":"gnome_extension","metadata":{"uuid":"e@x"}}"#,
        ));
        assert!(ext[0].path.contains("extensions/e@x/metadata.json"));
        let app = package_files(&item(r#"{"id":"org.x.A","type":"flatpak"}"#));
        assert!(app[0].path.ends_with("flatpak/app/org.x.A/"));
        let sys = package_files(&item(r#"{"id":"a","type":"system","package_name":"htop"}"#));
        assert_eq!(sys[0].path, "/usr/bin/htop");
    }

    #[test]
    fn every_role_has_an_icon() {
        for role in [
            FileRole::SkillPrompt,
            FileRole::Manifest,
            FileRole::PluginFiles,
            FileRole::ExtensionMetadata,
            FileRole::ExtensionCode,
            FileRole::ExtensionStyle,
            FileRole::ExtensionSchemas,
            FileRole::AppFiles,
            FileRole::DesktopEntry,
            FileRole::Binary,
        ] {
            assert!(role.icon().ends_with("-symbolic"));
        }
    }
}

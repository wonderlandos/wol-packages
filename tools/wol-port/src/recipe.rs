//! `recipes/<app>/recipe.toml`: where the source is, how it builds, what it needs, and the
//! `Info.json` fields of the resulting `.wol`.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FILE: &str = "recipe.toml";
/// WolOS App Store categories.
pub const CATEGORIES: [&str; 4] = ["Create", "Work", "Play", "Develop"];
/// Declared app capabilities (shown before install; WolOS `shared::package::CAPABILITIES`).
pub const CAPABILITIES: [&str; 9] = [
    "network",
    "files",
    "camera",
    "microphone",
    "location",
    "screen-capture",
    "notifications",
    "background",
    "devices",
];
pub const ARCH: &str = "amd64";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum System {
    Meson,
    Cmake,
    Cargo,
    Make,
    Autotools,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub git: String,
    pub tag: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Build {
    pub system: System,
    /// Extra configure arguments (`-Dx=y` for meson, `-DX=Y` for cmake, `--x` for configure).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// make(1) to use for make/autotools: BSD `make` unless this names e.g. `gmake` (then it
    /// must be a build dependency).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub make: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Deps {
    /// FreeBSD pkg names needed only to build.
    #[serde(default)]
    pub build: Vec<String>,
    /// FreeBSD pkg names needed at run time (ELF libraries are added automatically).
    #[serde(default)]
    pub run: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// Reverse-DNS app id (= desktop file and icon name upstream).
    pub id: String,
    /// pkg name.
    pub package: String,
    /// pkg version (`3.1.0`, `3.1.0_1`).
    pub version: String,
    pub name: String,
    pub summary: String,
    /// SPDX expression.
    pub license: String,
    pub homepage: String,
    pub categories: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Icon theme name.
    pub icon: String,
    /// Executable relative to `/usr/local`.
    pub entry: String,
    pub source: Source,
    pub build: Build,
    #[serde(default)]
    pub deps: Deps,
}

/// pkg name: `[A-Za-z0-9][A-Za-z0-9._+-]*`, at most 64.
pub fn is_pkg_name(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || b"._+-".contains(c))
}

/// pkg version: no `-`, starts alphanumeric.
pub fn is_pkg_version(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || b"._,+".contains(c))
}

pub fn is_app_id(s: &str) -> bool {
    s.contains('.')
        && s.len() <= 128
        && s.split('.').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
}

fn is_rel_path(s: &str) -> bool {
    !s.is_empty() && !s.starts_with('/') && s.split('/').all(|p| !p.is_empty() && p != "..")
}

/// Safe inside single quotes in the generated sh script and on one line.
fn is_shell_safe(s: &str) -> bool {
    !s.chars().any(|c| c == '\'' || c.is_control())
}

impl Recipe {
    pub fn parse(text: &str) -> Result<Recipe, String> {
        let r: Recipe = toml::from_str(text).map_err(|e| e.to_string())?;
        let errors = r.problems();
        if errors.is_empty() {
            Ok(r)
        } else {
            Err(errors.join("; "))
        }
    }

    pub fn load(dir: &std::path::Path) -> Result<Recipe, String> {
        let p = dir.join(FILE);
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        Recipe::parse(&text).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// Every rule the recipe breaks.
    pub fn problems(&self) -> Vec<String> {
        let mut e = Vec::new();
        let mut need = |ok: bool, what: &str| {
            if !ok {
                e.push(what.to_string());
            }
        };
        need(is_app_id(&self.id), "id must be reverse-DNS");
        need(is_pkg_name(&self.package), "package must be a pkg name");
        need(
            is_pkg_version(&self.version),
            "version must be a pkg version (no '-')",
        );
        need(
            !self.name.is_empty() && self.name.chars().count() <= 40,
            "name: 1..40 characters",
        );
        need(
            !self.summary.is_empty()
                && self.summary.chars().count() <= 80
                && !self.summary.ends_with('.'),
            "summary: 1..80 characters, no full stop",
        );
        need(!self.license.is_empty(), "license is required (SPDX)");
        need(
            self.homepage.starts_with("https://"),
            "homepage must be https",
        );
        need(
            !self.categories.is_empty()
                && self
                    .categories
                    .iter()
                    .all(|c| CATEGORIES.contains(&c.as_str())),
            "categories: one or more of Create, Work, Play, Develop",
        );
        need(
            self.capabilities
                .iter()
                .all(|c| CAPABILITIES.contains(&c.as_str())),
            "unknown capability",
        );
        need(
            !self.icon.is_empty()
                && self
                    .icon
                    .bytes()
                    .all(|b| !b.is_ascii_whitespace() && b != b'/'),
            "icon must be an icon theme name",
        );
        need(
            is_rel_path(&self.entry),
            "entry must be relative to /usr/local",
        );
        need(
            self.source.git.starts_with("https://") && is_shell_safe(&self.source.git),
            "source.git must be an https URL",
        );
        need(
            !self.source.tag.is_empty()
                && !self.source.tag.starts_with('-')
                && self
                    .source
                    .tag
                    .bytes()
                    .all(|b| b.is_ascii_graphic() && b != b'\''),
            "source.tag must be a plain tag",
        );
        need(
            self.build.options.iter().all(|o| is_shell_safe(o)),
            "build.options must not contain quotes or newlines",
        );
        need(
            self.build.env.iter().all(|(k, v)| {
                !k.is_empty()
                    && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    && !k.as_bytes()[0].is_ascii_digit()
                    && is_shell_safe(v)
            }),
            "build.env: NAME = 'value' without quotes",
        );
        need(
            self.deps
                .build
                .iter()
                .chain(&self.deps.run)
                .all(|d| is_pkg_name(d)),
            "deps must be pkg names",
        );
        if let Some(m) = &self.build.make {
            need(
                is_pkg_name(m) && (m == "make" || self.deps.build.contains(m)),
                "build.make must be listed in deps.build",
            );
        }
        e
    }

    /// pkg file name `pkg create` writes.
    pub fn pkg_file(&self) -> String {
        format!("{}-{}.pkg", self.package, self.version)
    }

    pub fn wol_file(&self) -> String {
        format!("{}-{}.wol", self.package, self.version)
    }

    /// `Info.json` (same bytes in the archive and in the payload).
    pub fn info_json(&self) -> String {
        #[derive(Serialize)]
        struct Target<'a> {
            abi: &'a str,
            arch: &'a str,
            entry: &'a str,
        }
        #[derive(Serialize)]
        struct Info<'a> {
            id: &'a str,
            version: &'a str,
            name: &'a str,
            summary: &'a str,
            icon: &'a str,
            categories: &'a [String],
            capabilities: &'a [String],
            package: &'a str,
            targets: [Target<'a>; 1],
        }
        let info = Info {
            id: &self.id,
            version: &self.version,
            name: &self.name,
            summary: &self.summary,
            icon: &self.icon,
            categories: &self.categories,
            capabilities: &self.capabilities,
            package: &self.package,
            targets: [Target {
                abi: "freebsd-native",
                arch: ARCH,
                entry: &self.entry,
            }],
        };
        let mut s = serde_json::to_string(&info).unwrap_or_default();
        s.push('\n');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const RAIDER: &str = r#"
id = "io.github.adbeveridge.Raider"
package = "raider"
version = "3.1.0"
name = "File Shredder"
summary = "Securely delete your files"
license = "GPL-3.0-or-later"
homepage = "https://github.com/ADBeveridge/raider"
categories = ["Work"]
capabilities = ["files"]
icon = "io.github.adbeveridge.Raider"
entry = "bin/raider"

[source]
git = "https://github.com/ADBeveridge/raider.git"
tag = "v3.1.0"

[build]
system = "meson"

[deps]
build = ["meson", "ninja", "pkgconf"]
run = ["gtk4", "libadwaita"]
"#;

    #[test]
    fn parses_a_good_recipe() {
        let r = Recipe::parse(RAIDER).unwrap();
        assert_eq!(r.build.system, System::Meson);
        assert_eq!(r.pkg_file(), "raider-3.1.0.pkg");
        assert_eq!(r.wol_file(), "raider-3.1.0.wol");
        assert_eq!(
            r.info_json(),
            "{\"id\":\"io.github.adbeveridge.Raider\",\"version\":\"3.1.0\",\"name\":\"File Shredder\",\"summary\":\"Securely delete your files\",\"icon\":\"io.github.adbeveridge.Raider\",\"categories\":[\"Work\"],\"capabilities\":[\"files\"],\"package\":\"raider\",\"targets\":[{\"abi\":\"freebsd-native\",\"arch\":\"amd64\",\"entry\":\"bin/raider\"}]}\n"
        );
    }

    #[test]
    fn rejects_table() {
        let cases = [
            (
                "id = \"io.github.adbeveridge.Raider\"",
                "id = \"raider\"",
                "reverse-DNS",
            ),
            ("package = \"raider\"", "package = \"-rf\"", "pkg name"),
            (
                "version = \"3.1.0\"",
                "version = \"3.1-beta\"",
                "pkg version",
            ),
            (
                "summary = \"Securely delete your files\"",
                "summary = \"Ends.\"",
                "full stop",
            ),
            ("homepage = \"https://", "homepage = \"http://", "https"),
            (
                "categories = [\"Work\"]",
                "categories = [\"Games\"]",
                "categories",
            ),
            (
                "capabilities = [\"files\"]",
                "capabilities = [\"root\"]",
                "capability",
            ),
            ("entry = \"bin/raider\"", "entry = \"/bin/sh\"", "entry"),
            ("entry = \"bin/raider\"", "entry = \"../x\"", "entry"),
            ("tag = \"v3.1.0\"", "tag = \"v1'; rm -rf /\"", "plain tag"),
            (
                "system = \"meson\"",
                "system = \"scons\"",
                "unknown variant",
            ),
            (
                "system = \"meson\"",
                "system = \"meson\"\noptions = [\"-Dx='y'\"]",
                "quotes",
            ),
            (
                "system = \"meson\"",
                "system = \"make\"\nmake = \"gmake\"",
                "deps.build",
            ),
            (
                "run = [\"gtk4\", \"libadwaita\"]",
                "run = [\"gtk4 libadwaita\"]",
                "pkg names",
            ),
            (
                "icon = \"io.github.adbeveridge.Raider\"",
                "icon = \"io.github.adbeveridge.Raider\"\nbogus = 1",
                "unknown field",
            ),
        ];
        for (from, to, want) in cases {
            let text = RAIDER.replacen(from, to, 1);
            assert_ne!(text, RAIDER, "{from}");
            let err = Recipe::parse(&text).unwrap_err();
            assert!(err.contains(want), "{to}: {err}");
        }
        let gmake = RAIDER
            .replacen(
                "system = \"meson\"",
                "system = \"make\"\nmake = \"gmake\"",
                1,
            )
            .replacen("\"pkgconf\"]", "\"pkgconf\", \"gmake\"]", 1);
        assert!(Recipe::parse(&gmake).is_ok());
    }

    #[test]
    fn name_rules() {
        for (s, ok) in [
            ("raider", true),
            ("py312-foo", true),
            ("-x", false),
            ("a b", false),
            ("", false),
        ] {
            assert_eq!(is_pkg_name(s), ok, "{s}");
        }
        for (s, ok) in [
            ("3.1.0", true),
            ("3.1.0_1", true),
            ("1.0,1", true),
            ("1-2", false),
            ("", false),
        ] {
            assert_eq!(is_pkg_version(s), ok, "{s}");
        }
        for (s, ok) in [
            ("org.gnome.Loupe", true),
            ("io.github.x-y.App_1", true),
            ("loupe", false),
            ("a..b", false),
            (".a.b", false),
        ] {
            assert_eq!(is_app_id(s), ok, "{s}");
        }
    }
}

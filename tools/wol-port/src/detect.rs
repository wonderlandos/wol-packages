//! `wol-port new`: inspect a source tree and draft a recipe.
use crate::depmap::Kind;
use crate::recipe::System;
use std::path::Path;

pub struct Draft {
    pub package: String,
    pub text: String,
    pub notes: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inspection {
    pub system: Option<System>,
    pub project: Option<String>,
    pub version: Option<String>,
    /// Upstream dependency names as found, in order, deduplicated.
    pub deps: Vec<(Kind, String)>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub license: Option<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    /// Python app (PyGObject): python3 and PyGObject are run dependencies.
    pub python: bool,
}

const SKIP_DIRS: [&str; 8] = [
    ".git",
    "subprojects",
    "build",
    "target",
    "po",
    "node_modules",
    "vendor",
    "tests",
];

fn walk(dir: &Path, depth: usize, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if p.is_dir() {
            if depth < 6 && !SKIP_DIRS.contains(&name) {
                walk(&p, depth + 1, out);
            }
        } else {
            out.push(p);
        }
    }
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Text inside `call( ... )` for every call (quotes respected, nested parentheses kept).
fn calls<'a>(text: &'a str, call: &str) -> Vec<&'a str> {
    let pat = format!("{call}(");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = text[from..].find(&pat) {
        let at = from + i;
        from = at + pat.len();
        if text[..at].chars().next_back().is_some_and(is_ident) {
            continue;
        }
        let (mut depth, mut quote) = (1, None);
        for (j, c) in text[from..].char_indices() {
            match (quote, c) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, '\'' | '"') => quote = Some(c),
                (None, '(') => depth += 1,
                (None, ')') => {
                    depth -= 1;
                    if depth == 0 {
                        out.push(&text[from..from + j]);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// First quoted string in `s`, if `s` starts with one (after blanks).
fn leading_string(s: &str) -> Option<String> {
    let s = s.trim_start();
    let q = s.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    let rest = &s[1..];
    Some(rest[..rest.find(q)?].to_string())
}

/// `key: 'value'` inside a meson call.
fn named(args: &str, key: &str) -> Option<String> {
    let pat = format!("{key}:");
    let i = args.find(&pat)?;
    leading_string(&args[i + pat.len()..])
}

fn plain(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.contains('@') && !s.contains('$'))
}

fn add(deps: &mut Vec<(Kind, String)>, k: Kind, n: &str) {
    let n = n.trim();
    if !n.is_empty() && !deps.iter().any(|(k2, n2)| *k2 == k && n2 == n) {
        deps.push((k, n.to_string()));
    }
}

/// Module names in a pkg-config list: `gtk4 >= 4.0 libxml-2.0`, `sqlite3>=3.30`.
fn pc_modules(list: &str) -> Vec<String> {
    list.split_whitespace()
        .map(|t| t.split(['<', '>', '=']).next().unwrap_or(""))
        .filter(|t| {
            !t.is_empty()
                && !t.starts_with(|c: char| c.is_ascii_digit())
                && !t.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        })
        .map(String::from)
        .collect()
}

fn xml_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let i = text.find(&open)? + open.len();
    let j = text[i..].find(&format!("</{tag}>"))?;
    plain(Some(text[i..i + j].to_string()))
}

fn desktop_key(text: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=");
    plain(
        text.lines()
            .find_map(|l| l.strip_prefix(&pat).map(String::from)),
    )
}

fn stem_id(file: &str, marker: &str) -> Option<String> {
    let id = file.split(marker).next()?;
    crate::recipe::is_app_id(id).then(|| id.to_string())
}

fn store_category(desktop_categories: &str) -> Option<&'static str> {
    let has = |c: &str| desktop_categories.split(';').any(|x| x == c);
    [
        ("Development", "Develop"),
        ("Game", "Play"),
        ("Graphics", "Create"),
        ("AudioVideo", "Play"),
        ("Audio", "Play"),
        ("Video", "Play"),
        ("Office", "Work"),
        ("Utility", "Work"),
        ("Network", "Work"),
        ("System", "Work"),
        ("Education", "Work"),
    ]
    .iter()
    .find(|(d, _)| has(d))
    .map(|(_, s)| *s)
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

fn cargo_deps(text: &str, deps: &mut Vec<(Kind, String)>) -> Option<String> {
    let v: toml::Table = toml::from_str(text).ok()?;
    if let Some(d) = v.get("dependencies").and_then(|d| d.as_table()) {
        for (k, spec) in d {
            let name = spec.get("package").and_then(|p| p.as_str()).unwrap_or(k);
            add(deps, Kind::Crate, name);
        }
    }
    v.get("package")?.get("name")?.as_str().map(String::from)
}

/// What a source tree builds with, needs, and calls itself.
pub fn inspect(dir: &Path) -> Inspection {
    let mut ins = Inspection::default();
    let mut files = Vec::new();
    walk(dir, 0, &mut files);
    let named_files = |n: &'static str| {
        files
            .iter()
            .filter(move |p| p.file_name().is_some_and(|f| f == n))
    };
    let root = |n: &str| dir.join(n).is_file();
    ins.system = if root("meson.build") {
        Some(System::Meson)
    } else if root("CMakeLists.txt") {
        Some(System::Cmake)
    } else if root("Cargo.toml") {
        Some(System::Cargo)
    } else if root("configure.ac") || root("configure.in") || root("configure") {
        Some(System::Autotools)
    } else if root("Makefile") || root("GNUmakefile") {
        Some(System::Make)
    } else {
        None
    };
    let mut app_id = None;
    // meson
    for p in named_files("meson.build") {
        let t = read(p);
        if p.parent() == Some(dir)
            && let Some(args) = calls(&t, "project").first()
        {
            ins.project = leading_string(args);
            ins.version = named(args, "version");
        }
        for a in calls(&t, "dependency") {
            if let Some(n) = leading_string(a) {
                add(&mut ins.deps, Kind::Pc, &n);
            }
        }
        for a in calls(&t, "find_program") {
            if let Some(n) = leading_string(a) {
                add(&mut ins.deps, Kind::Prog, &n);
            }
        }
        if t.contains("i18n.") || t.contains("import('i18n')") {
            add(&mut ins.deps, Kind::Prog, "msgfmt");
        }
        if app_id.is_none()
            && let Some(i) = t.find("application_id")
        {
            app_id = t[i..]
                .lines()
                .next()
                .and_then(|l| l.find(['\'', '"']).and_then(|q| leading_string(&l[q..])))
                .filter(|s| crate::recipe::is_app_id(s));
        }
    }
    // cmake
    for p in named_files("CMakeLists.txt") {
        let t = read(p);
        if p.parent() == Some(dir)
            && let Some(args) = calls(&t, "project").first()
        {
            let mut w = args.split_whitespace();
            ins.project = w.next().map(String::from);
            ins.version = args
                .split_whitespace()
                .skip_while(|x| *x != "VERSION")
                .nth(1)
                .map(String::from);
        }
        for a in calls(&t, "find_package") {
            if let Some(n) = a.split_whitespace().next() {
                add(&mut ins.deps, Kind::Cmake, n);
            }
        }
        for call in ["pkg_check_modules", "pkg_search_module"] {
            for a in calls(&t, call) {
                let rest = a.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
                for m in pc_modules(&rest) {
                    add(&mut ins.deps, Kind::Pc, &m);
                }
            }
        }
    }
    // autotools
    for n in ["configure.ac", "configure.in"] {
        let t = read(&dir.join(n));
        let unbracket = |s: &str| {
            s.trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string()
        };
        if let Some(a) = calls(&t, "AC_INIT").first() {
            let parts: Vec<String> = a.split(',').map(unbracket).collect();
            ins.project = parts.first().cloned().filter(|s| !s.is_empty());
            ins.version = parts.get(1).cloned().filter(|s| !s.is_empty());
        }
        for a in calls(&t, "PKG_CHECK_MODULES") {
            if let Some(list) = a.split(',').nth(1) {
                for m in pc_modules(&unbracket(list)) {
                    add(&mut ins.deps, Kind::Pc, &m);
                }
            }
        }
    }
    // cargo (alone or under meson)
    if let Some(name) = cargo_deps(&read(&dir.join("Cargo.toml")), &mut ins.deps)
        && ins.system == Some(System::Cargo)
    {
        ins.project = Some(name);
    }
    // Python apps: GObject introspection namespaces
    let mut python = files.iter().any(|p| {
        p.file_name().is_some_and(|f| f == "meson.build") && read(p).contains("import('python')")
    });
    for p in &files {
        let f = p.file_name().and_then(|f| f.to_str()).unwrap_or_default();
        if !(f.ends_with(".py") || f.ends_with(".in") || !f.contains('.')) {
            continue;
        }
        let t = read(p);
        for a in calls(&t, "require_version") {
            let mut parts = a.split(',').filter_map(leading_string);
            if let (Some(ns), Some(v)) = (parts.next(), parts.next()) {
                python = true;
                add(&mut ins.deps, Kind::Gi, &format!("{ns}-{v}"));
            }
        }
    }
    if python {
        add(&mut ins.deps, Kind::Prog, "python3");
    }
    ins.python = python;
    // app metadata: metainfo first, then desktop files
    let mut meta: Vec<_> = files.iter().collect();
    meta.sort_by_key(|p| {
        !p.to_string_lossy().contains("info.xml") && !p.to_string_lossy().contains("data.xml")
    });
    for p in meta {
        let f = p.file_name().and_then(|f| f.to_str()).unwrap_or_default();
        let t = || read(p);
        if f.contains(".metainfo.xml") || f.contains(".appdata.xml") {
            let t = t();
            let marker = if f.contains(".metainfo.xml") {
                ".metainfo.xml"
            } else {
                ".appdata.xml"
            };
            ins.id = ins.id.take().or_else(|| {
                xml_tag(&t, "id")
                    .filter(|i| crate::recipe::is_app_id(i))
                    .or_else(|| stem_id(f, marker))
            });
            ins.name = ins.name.take().or_else(|| xml_tag(&t, "name"));
            ins.summary = ins.summary.take().or_else(|| xml_tag(&t, "summary"));
            ins.license = ins
                .license
                .take()
                .or_else(|| xml_tag(&t, "project_license"));
        } else if f.contains(".desktop") && !f.ends_with(".po") {
            let t = t();
            if !t.contains("[Desktop Entry]") {
                continue;
            }
            ins.id = ins.id.take().or_else(|| stem_id(f, ".desktop"));
            ins.name = ins.name.take().or_else(|| desktop_key(&t, "Name"));
            ins.summary = ins.summary.take().or_else(|| desktop_key(&t, "Comment"));
            ins.icon = ins.icon.take().or_else(|| desktop_key(&t, "Icon"));
            if ins.categories.is_empty()
                && let Some(c) = desktop_key(&t, "Categories")
                    .as_deref()
                    .and_then(store_category)
            {
                ins.categories.push(c.into());
            }
        }
    }
    ins.id = ins.id.or(app_id);
    ins.icon = ins.icon.or_else(|| ins.id.clone());
    ins.summary = ins.summary.map(|s| {
        let s = s.trim_end_matches('.');
        s.chars().take(80).collect()
    });
    ins
}

fn version_key(tag: &str) -> Option<Vec<u64>> {
    let v = tag.trim_start_matches(|c: char| !c.is_ascii_digit());
    if v.is_empty() || !v.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    v.split('.').map(|p| p.parse().ok()).collect()
}

/// Newest release tag from `git ls-remote --tags --refs` output (pre-releases skipped).
pub fn latest_tag(ls_remote: &str) -> Option<String> {
    ls_remote
        .lines()
        .filter_map(|l| l.split("refs/tags/").nth(1))
        .filter_map(|t| version_key(t).map(|k| (k, t.to_string())))
        .max()
        .map(|(_, t)| t)
}

/// Packages that are only needed to build.
const BUILD_ONLY: [&str; 19] = [
    "pkgconf",
    "meson",
    "ninja",
    "cmake",
    "rust",
    "gettext-tools",
    "blueprint-compiler",
    "desktop-file-utils",
    "AppStream",
    "appstream-glib",
    "vala",
    "gobject-introspection",
    "itstool",
    "sassc",
    "gperf",
    "bison",
    "git",
    "gmake",
    "libxslt",
];

pub fn draft(ins: &Inspection, url: &str, tag: &str) -> Draft {
    use crate::depmap::{Mapped, map};
    use crate::recipe::{Build, Deps, Recipe, Source, is_pkg_version};
    let repo = url.trim_end_matches('/').trim_end_matches(".git");
    let package: String = repo
        .rsplit('/')
        .next()
        .unwrap_or("app")
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._+".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect();
    let mut notes = Vec::new();
    let from_tag = tag
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .replace(['-', '_'], ".");
    let version = if is_pkg_version(&from_tag) {
        from_tag
    } else {
        ins.version.clone().unwrap_or_else(|| "0".into())
    };
    let (mut build, mut run) = (Vec::new(), Vec::new());
    for (k, n) in &ins.deps {
        match map(*k, n) {
            Mapped::Pkg(p) if *k == Kind::Prog || BUILD_ONLY.contains(&p) => {
                build.push(p.to_string())
            }
            Mapped::Pkg(p) => run.push(p.to_string()),
            Mapped::LinuxOnly => notes.push(format!(
                "Linux only: {n} ({k:?}): patch it out or disable the option"
            )),
            Mapped::Unknown => {
                notes.push(format!("unmapped: {n} ({k:?}): find the FreeBSD package"))
            }
            Mapped::Base | Mapped::Ignore => {}
        }
    }
    if ins.python {
        run.extend(["python3".to_string(), "py312-pygobject".to_string()]);
    }
    run.sort();
    run.dedup();
    build.retain(|b| !run.contains(b));
    build.sort();
    build.dedup();
    let project = ins.project.clone().unwrap_or_else(|| package.clone());
    let entry = format!("bin/{}", project.to_lowercase());
    let r = Recipe {
        id: ins.id.clone().unwrap_or_default(),
        package: package.clone(),
        version,
        name: ins.name.clone().unwrap_or(project),
        summary: ins.summary.clone().unwrap_or_default(),
        license: ins.license.clone().unwrap_or_default(),
        homepage: if repo.starts_with("https://") {
            repo.to_string()
        } else {
            String::new()
        },
        categories: ins.categories.clone(),
        capabilities: Vec::new(),
        icon: ins.icon.clone().unwrap_or_default(),
        entry: entry.clone(),
        source: Source {
            git: url.to_string(),
            tag: tag.to_string(),
        },
        build: Build {
            system: ins.system.unwrap_or(System::Make),
            options: Vec::new(),
            env: Default::default(),
            make: None,
        },
        deps: Deps { build, run },
    };
    let missing: Vec<&str> = [
        ("id", r.id.is_empty()),
        ("summary", r.summary.is_empty()),
        ("license", r.license.is_empty()),
        ("categories", r.categories.is_empty()),
        ("icon", r.icon.is_empty()),
    ]
    .iter()
    .filter(|(_, m)| *m)
    .map(|(n, _)| *n)
    .collect();
    if !missing.is_empty() {
        notes.insert(0, format!("fill in: {}", missing.join(", ")));
    }
    if ins.system.is_none() {
        notes.push("no known build system found: set build.system".into());
    }
    notes.push(format!("check entry = \"{entry}\" and capabilities"));
    let mut text = toml::to_string(&r).unwrap_or_default();
    text.push_str("\n# Drafted by wol-port new. Notes:\n");
    for n in &notes {
        text.push_str(&format!("# - {n}\n"));
    }
    Draft {
        package,
        text,
        notes: notes.iter().map(|n| format!("{n}\n")).collect(),
    }
}

fn git(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Shallow-clone `url` at `tag` (default: newest release tag), inspect, draft.
pub fn new_recipe(url: &str, tag: Option<&str>) -> Result<Draft, String> {
    if !url.starts_with("https://") {
        return Err("use an https git URL".into());
    }
    let tag = match tag {
        Some(t) => t.to_string(),
        None => latest_tag(&git(&["ls-remote", "--tags", "--refs", url])?)
            .ok_or("no release tag found; pass --tag")?,
    };
    let dir = std::env::temp_dir().join(format!("wol-port-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let d = dir.to_string_lossy().to_string();
    let r = git(&["clone", "-q", "--depth", "1", "--branch", &tag, url, &d])
        .map(|_| draft(&inspect(&dir), url, &tag));
    let _ = std::fs::remove_dir_all(&dir);
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::depmap::Kind::*;
    use crate::recipe::Recipe;

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn has(ins: &Inspection, k: Kind, n: &str) -> bool {
        ins.deps.iter().any(|(k2, n2)| *k2 == k && n2 == n)
    }

    #[test]
    fn inspects_fixture_projects() {
        let m = inspect(&fixture("meson-c"));
        assert_eq!(m.system, Some(System::Meson));
        assert_eq!(
            (m.project.as_deref(), m.version.as_deref()),
            (Some("shredder"), Some("3.1.0"))
        );
        assert_eq!(m.id.as_deref(), Some("org.example.Shredder"));
        assert_eq!(m.name.as_deref(), Some("File Shredder"));
        assert_eq!(m.summary.as_deref(), Some("Securely delete your files"));
        assert_eq!(m.license.as_deref(), Some("GPL-3.0-or-later"));
        assert_eq!(m.icon.as_deref(), Some("org.example.Shredder"));
        assert_eq!(m.categories, ["Work"]);
        for (k, n) in [
            (Pc, "gtk4"),
            (Pc, "libadwaita-1"),
            (Pc, "gio-unix-2.0"),
            (Pc, "libsystemd"),
            (Prog, "blueprint-compiler"),
            (Prog, "desktop-file-validate"),
            (Prog, "msgfmt"),
        ] {
            assert!(has(&m, k, n), "meson-c {k:?} {n}: {:?}", m.deps);
        }

        let r = inspect(&fixture("meson-rust"));
        assert_eq!(r.system, Some(System::Meson));
        assert_eq!(r.id.as_deref(), Some("dev.example.Keytap"));
        assert_eq!(r.summary.as_deref(), Some("Learn to type"));
        assert_eq!(r.categories, ["Play"]);
        for (k, n) in [
            (Crate, "gtk4"),
            (Crate, "libadwaita"),
            (Crate, "gstreamer-play-sys"),
            (Crate, "rand"),
            (Prog, "cargo"),
        ] {
            assert!(has(&r, k, n), "meson-rust {k:?} {n}: {:?}", r.deps);
        }

        let c = inspect(&fixture("cmake-qt"));
        assert_eq!(c.system, Some(System::Cmake));
        assert_eq!(
            (c.project.as_deref(), c.version.as_deref()),
            (Some("Notes"), Some("2.4.1"))
        );
        for (k, n) in [
            (Cmake, "Qt6"),
            (Cmake, "ZLIB"),
            (Cmake, "PkgConfig"),
            (Pc, "libsecret-1"),
            (Pc, "sqlite3"),
        ] {
            assert!(has(&c, k, n), "cmake {k:?} {n}: {:?}", c.deps);
        }

        let a = inspect(&fixture("autotools"));
        assert_eq!(a.system, Some(System::Autotools));
        assert_eq!(
            (a.project.as_deref(), a.version.as_deref()),
            (Some("tinyapp"), Some("0.9.2"))
        );
        for n in ["gtk+-3.0", "libxml-2.0", "libcurl"] {
            assert!(has(&a, Pc, n), "autotools {n}: {:?}", a.deps);
        }
        assert!(
            !a.deps
                .iter()
                .any(|(_, n)| n.contains('>') || n.starts_with('3'))
        );

        let py = inspect(&fixture("meson-python"));
        for (k, n) in [
            (Gi, "Gtk-4.0"),
            (Gi, "Adw-1"),
            (Gi, "GstPlay-1.0"),
            (Gi, "WebKit-6.0"),
            (Prog, "python3"),
            (Prog, "msgfmt"),
        ] {
            assert!(has(&py, k, n), "python {k:?} {n}: {:?}", py.deps);
        }
        let d = draft(&py, "https://github.com/x/soundy", "0.8.0");
        let r: Recipe = toml::from_str(&d.text).unwrap();
        assert_eq!(
            r.deps.run,
            [
                "gstreamer1-plugins-bad",
                "gtk4",
                "libadwaita",
                "py312-pygobject",
                "python3",
                "webkit2-gtk_60"
            ]
        );

        let g = inspect(&fixture("cargo-only"));
        assert_eq!(g.system, Some(System::Cargo));
        assert_eq!(g.project.as_deref(), Some("clitool"));
        assert!(has(&g, Crate, "libudev-sys") && has(&g, Crate, "anyhow"));
    }

    #[test]
    fn drafts_a_valid_recipe_with_notes() {
        let d = draft(
            &inspect(&fixture("meson-c")),
            "https://github.com/example/Shredder.git",
            "v3.1.0",
        );
        assert_eq!(d.package, "shredder");
        let r = Recipe::parse(&d.text).unwrap_or_else(|e| panic!("{e}\n{}", d.text));
        assert_eq!(r.version, "3.1.0");
        assert_eq!(r.entry, "bin/shredder");
        assert_eq!(r.homepage, "https://github.com/example/Shredder");
        assert_eq!(r.deps.run, ["glib", "gtk4", "libadwaita"]);
        assert_eq!(
            r.deps.build,
            ["blueprint-compiler", "desktop-file-utils", "gettext-tools"]
        );
        assert!(d.notes.contains("Linux only: libsystemd"), "{}", d.notes);

        let d = draft(
            &inspect(&fixture("meson-rust")),
            "https://github.com/x/keytap",
            "7.0",
        );
        assert!(d.text.contains("license = \"\""), "{}", d.text);
        assert!(d.notes.contains("fill in: license"), "{}", d.notes);
        let r: Recipe = toml::from_str(&d.text).unwrap();
        assert_eq!(r.deps.run, ["gstreamer1-plugins-bad", "gtk4", "libadwaita"]);
        assert!(r.deps.build.contains(&"rust".to_string()));

        let d = draft(
            &inspect(&fixture("cmake-qt")),
            "https://gitlab.com/x/notes.git",
            "2.4.1",
        );
        let r: Recipe = toml::from_str(&d.text).unwrap();
        assert_eq!(r.deps.run, ["libsecret", "qt6-base", "sqlite3"]);
        assert_eq!(r.deps.build, ["pkgconf"]);
        assert!(d.notes.contains("fill in: id"), "{}", d.notes);
    }

    #[test]
    fn picks_the_newest_release_tag() {
        let out = "a\trefs/tags/v1.9.0\nb\trefs/tags/v1.10.0\nc\trefs/tags/v2.0.0-rc1\nd\trefs/tags/nightly\ne\trefs/tags/1.2\n";
        assert_eq!(latest_tag(out).as_deref(), Some("v1.10.0"));
        assert_eq!(latest_tag("x\trefs/tags/nightly\n"), None);
    }
}

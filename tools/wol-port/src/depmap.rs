//! Upstream dependency names (pkg-config modules, programs, CMake packages, Rust crates, Debian
//! packages) -> FreeBSD pkg names. One table; `wol-port map` prints lookups.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// pkg-config module (meson `dependency()`, `pkg_check_modules`, `PKG_CHECK_MODULES`).
    Pc,
    /// Program (meson `find_program`).
    Prog,
    /// CMake `find_package`.
    Cmake,
    /// Rust crate in Cargo.toml.
    Crate,
    /// Debian/Ubuntu package (README build instructions).
    Deb,
    /// GObject introspection namespace `Name-version` (Python `gi.require_version`).
    Gi,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "pc" => Kind::Pc,
            "prog" => Kind::Prog,
            "cmake" => Kind::Cmake,
            "crate" => Kind::Crate,
            "deb" => Kind::Deb,
            "gi" => Kind::Gi,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mapped {
    Pkg(&'static str),
    /// In FreeBSD base: nothing to install.
    Base,
    /// Linux-only interface: the app needs a patch or an option to drop it.
    LinuxOnly,
    /// Not a dependency worth tracking (pure-Rust crates, build helpers).
    Ignore,
    Unknown,
}

impl Mapped {
    pub fn describe(self) -> &'static str {
        match self {
            Mapped::Pkg(p) => p,
            Mapped::Base => "(FreeBSD base)",
            Mapped::LinuxOnly => "(Linux only: patch or disable)",
            Mapped::Ignore => "(nothing to install)",
            Mapped::Unknown => "(unmapped)",
        }
    }
}

use Kind::*;
use Mapped::{Base, Ignore, LinuxOnly, Pkg};

/// (kind, upstream name, FreeBSD). Names in the quarterly FreeBSD:15:amd64 catalogue.
pub const TABLE: &[(Kind, &str, Mapped)] = &[
    // pkg-config
    (Pc, "gtk4", Pkg("gtk4")),
    (Pc, "gtk4-wayland", Pkg("gtk4")),
    (Pc, "gtk+-3.0", Pkg("gtk3")),
    (Pc, "libadwaita-1", Pkg("libadwaita")),
    (Pc, "glib-2.0", Pkg("glib")),
    (Pc, "gio-2.0", Pkg("glib")),
    (Pc, "gio-unix-2.0", Pkg("glib")),
    (Pc, "gobject-2.0", Pkg("glib")),
    (Pc, "gmodule-2.0", Pkg("glib")),
    (Pc, "gthread-2.0", Pkg("glib")),
    (
        Pc,
        "gobject-introspection-1.0",
        Pkg("gobject-introspection"),
    ),
    (Pc, "gtksourceview-5", Pkg("gtksourceview5")),
    (Pc, "webkitgtk-6.0", Pkg("webkit2-gtk_60")),
    (Pc, "javascriptcoregtk-6.0", Pkg("webkit2-gtk_60")),
    (Pc, "webkit2gtk-4.1", Pkg("webkit2-gtk_41")),
    (Pc, "webkit2gtk-4.0", Pkg("webkit2-gtk_40")),
    (Pc, "libsoup-3.0", Pkg("libsoup3")),
    (Pc, "json-glib-1.0", Pkg("json-glib")),
    (Pc, "libsecret-1", Pkg("libsecret")),
    (Pc, "libportal", Pkg("libportal")),
    (Pc, "libportal-gtk4", Pkg("libportal-gtk4")),
    (Pc, "poppler-glib", Pkg("poppler-glib")),
    (Pc, "libxml-2.0", Pkg("libxml2")),
    (Pc, "sqlite3", Pkg("sqlite3")),
    (Pc, "libcurl", Pkg("curl")),
    (Pc, "cairo", Pkg("cairo")),
    (Pc, "pango", Pkg("pango")),
    (Pc, "pangocairo", Pkg("pango")),
    (Pc, "gdk-pixbuf-2.0", Pkg("gdk-pixbuf2")),
    (Pc, "graphene-gobject-1.0", Pkg("graphene")),
    (Pc, "gstreamer-1.0", Pkg("gstreamer1")),
    (Pc, "gstreamer-base-1.0", Pkg("gstreamer1")),
    (Pc, "gstreamer-plugins-base-1.0", Pkg("gstreamer1-plugins")),
    (Pc, "gstreamer-audio-1.0", Pkg("gstreamer1-plugins")),
    (Pc, "gstreamer-video-1.0", Pkg("gstreamer1-plugins")),
    (Pc, "gstreamer-pbutils-1.0", Pkg("gstreamer1-plugins")),
    (Pc, "gstreamer-tag-1.0", Pkg("gstreamer1-plugins")),
    (Pc, "gstreamer-play-1.0", Pkg("gstreamer1-plugins-bad")),
    (Pc, "gstreamer-player-1.0", Pkg("gstreamer1-plugins-bad")),
    (Pc, "libpulse", Pkg("pulseaudio")),
    (Pc, "libpipewire-0.3", Pkg("pipewire")),
    (Pc, "alsa", Pkg("alsa-lib")),
    (Pc, "x11", Pkg("libX11")),
    (Pc, "wayland-client", Pkg("wayland")),
    (Pc, "vulkan", Pkg("vulkan-loader")),
    (Pc, "epoxy", Pkg("libepoxy")),
    (Pc, "gee-0.8", Pkg("libgee")),
    (Pc, "libspelling-1", Pkg("libspelling")),
    (Pc, "shumate-1.0", Pkg("libshumate")),
    (Pc, "libnotify", Pkg("libnotify")),
    (Pc, "libjpeg", Pkg("jpeg-turbo")),
    (Pc, "libpng", Pkg("png")),
    (Pc, "libwebp", Pkg("webp")),
    (Pc, "libheif", Pkg("libheif")),
    (Pc, "lcms2", Pkg("lcms2")),
    (Pc, "librsvg-2.0", Pkg("librsvg2-rust")),
    (Pc, "fontconfig", Pkg("fontconfig")),
    (Pc, "freetype2", Pkg("freetype2")),
    (Pc, "harfbuzz", Pkg("harfbuzz")),
    (Pc, "libzstd", Pkg("zstd")),
    (Pc, "libpcre2-8", Pkg("pcre2")),
    (Pc, "libical-glib", Pkg("libical")),
    (Pc, "libavcodec", Pkg("ffmpeg")),
    (Pc, "libavformat", Pkg("ffmpeg")),
    (Pc, "libavutil", Pkg("ffmpeg")),
    (Pc, "zbar", Pkg("zbar")),
    (Pc, "dbus-1", Pkg("dbus")),
    (Pc, "libudev", Pkg("libudev-devd")),
    (Pc, "gudev-1.0", Pkg("libgudev")),
    (Pc, "libevdev", Pkg("libevdev")),
    (Pc, "zlib", Base),
    (Pc, "openssl", Base),
    (Pc, "libssl", Base),
    (Pc, "libcrypto", Base),
    (Pc, "libarchive", Base),
    (Pc, "libusb-1.0", Base),
    (Pc, "ncurses", Base),
    (Pc, "libsystemd", LinuxOnly),
    (Pc, "libseccomp", LinuxOnly),
    (Pc, "libcap", LinuxOnly),
    (Pc, "libselinux", LinuxOnly),
    (Pc, "libnm", LinuxOnly),
    (Pc, "libbpf", LinuxOnly),
    // programs
    (Prog, "blueprint-compiler", Pkg("blueprint-compiler")),
    (Prog, "glib-compile-resources", Pkg("glib")),
    (Prog, "glib-compile-schemas", Pkg("glib")),
    (Prog, "gdbus-codegen", Pkg("glib")),
    (Prog, "glib-mkenums", Pkg("glib")),
    (Prog, "desktop-file-validate", Pkg("desktop-file-utils")),
    (Prog, "update-desktop-database", Pkg("desktop-file-utils")),
    (Prog, "appstreamcli", Pkg("AppStream")),
    (Prog, "appstream-util", Pkg("appstream-glib")),
    (Prog, "gtk-update-icon-cache", Pkg("gtk-update-icon-cache")),
    (Prog, "gtk4-update-icon-cache", Pkg("gtk4")),
    (Prog, "msgfmt", Pkg("gettext-tools")),
    (Prog, "xgettext", Pkg("gettext-tools")),
    (Prog, "msgmerge", Pkg("gettext-tools")),
    (Prog, "itstool", Pkg("itstool")),
    (Prog, "sassc", Pkg("sassc")),
    (Prog, "cargo", Pkg("rust")),
    (Prog, "rustc", Pkg("rust")),
    (Prog, "valac", Pkg("vala")),
    (Prog, "python3", Pkg("python3")),
    (Prog, "python", Pkg("python3")),
    (Prog, "g-ir-scanner", Pkg("gobject-introspection")),
    (Prog, "xmllint", Pkg("libxml2")),
    (Prog, "xsltproc", Pkg("libxslt")),
    (Prog, "git", Pkg("git")),
    (Prog, "gperf", Pkg("gperf")),
    (Prog, "bison", Pkg("bison")),
    (Prog, "perl", Pkg("perl5")),
    (Prog, "pandoc", Pkg("hs-pandoc")),
    (Prog, "gjs", Pkg("gjs")),
    (Prog, "meson", Pkg("meson")),
    (Prog, "ninja", Pkg("ninja")),
    (Prog, "cmake", Pkg("cmake")),
    (Prog, "pkg-config", Pkg("pkgconf")),
    (Prog, "gmake", Pkg("gmake")),
    (Prog, "bash", Pkg("bash")),
    (Prog, "make", Base),
    (Prog, "m4", Base),
    (Prog, "flex", Base),
    (Prog, "yacc", Base),
    (Prog, "sh", Base),
    (Prog, "tar", Base),
    // CMake packages
    (Cmake, "Qt6", Pkg("qt6-base")),
    (Cmake, "Qt5", Pkg("qt5-core")),
    (Cmake, "PkgConfig", Pkg("pkgconf")),
    (Cmake, "Gettext", Pkg("gettext-tools")),
    (Cmake, "Intl", Pkg("gettext-runtime")),
    (Cmake, "CURL", Pkg("curl")),
    (Cmake, "SQLite3", Pkg("sqlite3")),
    (Cmake, "PNG", Pkg("png")),
    (Cmake, "JPEG", Pkg("jpeg-turbo")),
    (Cmake, "Freetype", Pkg("freetype2")),
    (Cmake, "Boost", Pkg("boost-libs")),
    (Cmake, "SDL2", Pkg("sdl2")),
    (Cmake, "SDL3", Pkg("sdl3")),
    (Cmake, "Vulkan", Pkg("vulkan-loader")),
    (Cmake, "X11", Pkg("libX11")),
    (Cmake, "LibXml2", Pkg("libxml2")),
    (Cmake, "EXPAT", Pkg("expat")),
    (Cmake, "fmt", Pkg("libfmt")),
    (Cmake, "spdlog", Pkg("spdlog")),
    (Cmake, "nlohmann_json", Pkg("nlohmann-json")),
    (Cmake, "Python3", Pkg("python3")),
    (Cmake, "ZLIB", Base),
    (Cmake, "OpenSSL", Base),
    (Cmake, "Threads", Base),
    (Cmake, "Libsystemd", LinuxOnly),
    // Rust crates
    (Crate, "gtk4", Pkg("gtk4")),
    (Crate, "gtk", Pkg("gtk4")),
    (Crate, "libadwaita", Pkg("libadwaita")),
    (Crate, "adw", Pkg("libadwaita")),
    (Crate, "glib", Pkg("glib")),
    (Crate, "gio", Pkg("glib")),
    (Crate, "gstreamer", Pkg("gstreamer1")),
    (Crate, "gstreamer-pbutils", Pkg("gstreamer1-plugins")),
    (Crate, "gstreamer-audio", Pkg("gstreamer1-plugins")),
    (Crate, "gstreamer-video", Pkg("gstreamer1-plugins")),
    (Crate, "gstreamer-play", Pkg("gstreamer1-plugins-bad")),
    (Crate, "gstreamer-player", Pkg("gstreamer1-plugins-bad")),
    (Crate, "sourceview5", Pkg("gtksourceview5")),
    (Crate, "webkit6", Pkg("webkit2-gtk_60")),
    (Crate, "soup3", Pkg("libsoup3")),
    (Crate, "libshumate", Pkg("libshumate")),
    (Crate, "poppler-rs", Pkg("poppler-glib")),
    (Crate, "libspelling", Pkg("libspelling")),
    (Crate, "libdbus-sys", Pkg("dbus")),
    (Crate, "libudev-sys", Pkg("libudev-devd")),
    (Crate, "pipewire", Pkg("pipewire")),
    (Crate, "libpulse-binding", Pkg("pulseaudio")),
    (Crate, "alsa-sys", Pkg("alsa-lib")),
    (Crate, "openssl-sys", Base),
    (Crate, "libseccomp", LinuxOnly),
    (Crate, "libsystemd", LinuxOnly),
    (Crate, "procfs", LinuxOnly),
    (Crate, "glycin", LinuxOnly),
    // Debian/Ubuntu
    (Deb, "libgtk-4-dev", Pkg("gtk4")),
    (Deb, "libadwaita-1-dev", Pkg("libadwaita")),
    (Deb, "libglib2.0-dev", Pkg("glib")),
    (Deb, "libgstreamer1.0-dev", Pkg("gstreamer1")),
    (
        Deb,
        "libgstreamer-plugins-base1.0-dev",
        Pkg("gstreamer1-plugins"),
    ),
    (Deb, "libsoup-3.0-dev", Pkg("libsoup3")),
    (Deb, "libwebkitgtk-6.0-dev", Pkg("webkit2-gtk_60")),
    (Deb, "libjson-glib-dev", Pkg("json-glib")),
    (Deb, "libsecret-1-dev", Pkg("libsecret")),
    (Deb, "libgtksourceview-5-dev", Pkg("gtksourceview5")),
    (Deb, "libportal-gtk4-dev", Pkg("libportal-gtk4")),
    (Deb, "libpoppler-glib-dev", Pkg("poppler-glib")),
    (Deb, "libsqlite3-dev", Pkg("sqlite3")),
    (Deb, "libcurl4-openssl-dev", Pkg("curl")),
    (Deb, "ninja-build", Pkg("ninja")),
    (Deb, "meson", Pkg("meson")),
    (Deb, "blueprint-compiler", Pkg("blueprint-compiler")),
    (Deb, "gettext", Pkg("gettext-tools")),
    (Deb, "desktop-file-utils", Pkg("desktop-file-utils")),
    (Deb, "appstream", Pkg("AppStream")),
    (Deb, "python3-gi", Pkg("py312-pygobject")),
    (Deb, "cargo", Pkg("rust")),
    (Deb, "rustc", Pkg("rust")),
    (Deb, "valac", Pkg("vala")),
    (Deb, "libssl-dev", Base),
    (Deb, "zlib1g-dev", Base),
    (Deb, "build-essential", Base),
    (Deb, "libsystemd-dev", LinuxOnly),
    (Deb, "libseccomp-dev", LinuxOnly),
    // GObject introspection namespaces (typelibs)
    (Gi, "Gtk-4.0", Pkg("gtk4")),
    (Gi, "Gdk-4.0", Pkg("gtk4")),
    (Gi, "Gsk-4.0", Pkg("gtk4")),
    (Gi, "Gtk-3.0", Pkg("gtk3")),
    (Gi, "Adw-1", Pkg("libadwaita")),
    (Gi, "GLib-2.0", Pkg("glib")),
    (Gi, "Gio-2.0", Pkg("glib")),
    (Gi, "GObject-2.0", Pkg("glib")),
    (Gi, "Pango-1.0", Pkg("pango")),
    (Gi, "GdkPixbuf-2.0", Pkg("gdk-pixbuf2")),
    (Gi, "Graphene-1.0", Pkg("graphene")),
    (Gi, "Gst-1.0", Pkg("gstreamer1")),
    (Gi, "GstPbutils-1.0", Pkg("gstreamer1-plugins")),
    (Gi, "GstAudio-1.0", Pkg("gstreamer1-plugins")),
    (Gi, "GstVideo-1.0", Pkg("gstreamer1-plugins")),
    (Gi, "GstPlay-1.0", Pkg("gstreamer1-plugins-bad")),
    (Gi, "WebKit-6.0", Pkg("webkit2-gtk_60")),
    (Gi, "Soup-3.0", Pkg("libsoup3")),
    (Gi, "GtkSource-5", Pkg("gtksourceview5")),
    (Gi, "Secret-1", Pkg("libsecret")),
    (Gi, "Xdp-1.0", Pkg("libportal")),
    (Gi, "XdpGtk4-1.0", Pkg("libportal-gtk4")),
    (Gi, "Shumate-1.0", Pkg("libshumate")),
    (Gi, "Spelling-1", Pkg("libspelling")),
    (Gi, "Poppler-0.18", Pkg("poppler-glib")),
    (Gi, "Json-1.0", Pkg("json-glib")),
    (Gi, "Notify-0.7", Pkg("libnotify")),
    (Gi, "Rsvg-2.0", Pkg("librsvg2-rust")),
    // late additions
    (Prog, "gsed", Pkg("gsed")),
    (Prog, "sed", Base),
    (Pc, "exempi-2.0", Pkg("exempi")),
    (Pc, "intl", Pkg("gettext-runtime")),
    (Pc, "threads", Base),
    (Pc, "dl", Base),
];

/// Pure-Rust crates that never need a system package.
fn ignorable_crate(name: &str) -> bool {
    !name.ends_with("-sys")
}

pub fn map(kind: Kind, name: &str) -> Mapped {
    let name = name.trim();
    let hit = TABLE.iter().find(|(k, n, _)| {
        *k == kind
            && match kind {
                Cmake | Deb => n.eq_ignore_ascii_case(name),
                _ => *n == name,
            }
    });
    match (hit, kind) {
        (Some((_, _, m)), _) => *m,
        // gtk-rs and friends name system crates `<lib>-sys`: map the base name.
        (None, Crate) if name.ends_with("-sys") => match map(Crate, &name[..name.len() - 4]) {
            Mapped::Ignore => Mapped::Unknown,
            m => m,
        },
        (None, Crate) if ignorable_crate(name) => Ignore,
        (None, Kind::Cmake) if name.starts_with("KF6") || name.starts_with("Qt6") => {
            Pkg("qt6-base")
        }
        _ => Mapped::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_table() {
        let cases = [
            (Pc, "gtk4", Pkg("gtk4")),
            (Pc, "libadwaita-1", Pkg("libadwaita")),
            (Pc, "gio-unix-2.0", Pkg("glib")),
            (Pc, "webkitgtk-6.0", Pkg("webkit2-gtk_60")),
            (Pc, "zlib", Base),
            (Pc, "libsystemd", LinuxOnly),
            (Pc, "libseccomp", LinuxOnly),
            (Pc, "nonexistent-9", Mapped::Unknown),
            (Prog, "blueprint-compiler", Pkg("blueprint-compiler")),
            (Prog, "msgfmt", Pkg("gettext-tools")),
            (Prog, "cargo", Pkg("rust")),
            (Prog, "make", Base),
            (Cmake, "qt6", Pkg("qt6-base")),
            (Cmake, "Qt6Widgets", Pkg("qt6-base")),
            (Cmake, "ZLIB", Base),
            (Crate, "gtk4", Pkg("gtk4")),
            (Crate, "libadwaita", Pkg("libadwaita")),
            (Crate, "gstreamer-play-sys", Pkg("gstreamer1-plugins-bad")),
            (Crate, "openssl-sys", Base),
            (Crate, "libseccomp", LinuxOnly),
            (Crate, "serde", Ignore),
            (Crate, "mystery-sys", Mapped::Unknown),
            (Deb, "libgtk-4-dev", Pkg("gtk4")),
            (Deb, "LIBSYSTEMD-DEV", LinuxOnly),
            (Prog, "gsed", Pkg("gsed")),
            (Pc, "exempi-2.0", Pkg("exempi")),
            (Pc, "intl", Pkg("gettext-runtime")),
            (Pc, "threads", Base),
            (Gi, "Gtk-4.0", Pkg("gtk4")),
            (Gi, "Adw-1", Pkg("libadwaita")),
            (Gi, "WebKit-6.0", Pkg("webkit2-gtk_60")),
            (Gi, "GstPlay-1.0", Pkg("gstreamer1-plugins-bad")),
            (Gi, "GLib-2.0", Pkg("glib")),
        ];
        for (k, n, want) in cases {
            assert_eq!(map(k, n), want, "{k:?} {n}");
        }
    }

    #[test]
    fn table_is_clean() {
        for (i, (k, n, m)) in TABLE.iter().enumerate() {
            if let Pkg(p) = m {
                assert!(crate::recipe::is_pkg_name(p), "{p}");
            }
            assert!(
                !TABLE[..i].iter().any(|(k2, n2, _)| k2 == k && n2 == n),
                "duplicate {k:?} {n}"
            );
        }
        assert_eq!(Kind::parse("pc"), Some(Pc));
        assert_eq!(Kind::parse("rpm"), None);
        assert_eq!(map(Pc, "gtk4").describe(), "gtk4");
        assert_eq!(map(Pc, "x").describe(), "(unmapped)");
    }
}

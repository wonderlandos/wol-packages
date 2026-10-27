//! `index.json`: every published `.wol`, newest version per id (archicture/modules/packages.md).
use crate::archive::{Wol, sha256_hex};
use crate::recipe::{is_app_id, is_pkg_name, is_pkg_version};
use serde::{Deserialize, Serialize};

pub const FORMAT: u32 = 1;
pub const ABI: &str = "FreeBSD:15:amd64";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub version: String,
    pub package: String,
    pub categories: Vec<String>,
    pub capabilities: Vec<String>,
    pub abi: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    /// Icon URL (`<icons>/<id>.png`) when the `.wol` carries `icon.png`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    pub signed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub format: u32,
    pub packages: Vec<Entry>,
}

impl Default for Index {
    fn default() -> Self {
        Index {
            format: FORMAT,
            packages: Vec::new(),
        }
    }
}

/// The `Info.json` fields the index needs.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InfoFields {
    pub id: String,
    pub version: String,
    pub package: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

pub fn info_of(json: &[u8]) -> Result<InfoFields, String> {
    let i: InfoFields = serde_json::from_slice(json).map_err(|e| format!("Info.json: {e}"))?;
    if !is_app_id(&i.id) || !is_pkg_name(&i.package) || !is_pkg_version(&i.version) {
        return Err("Info.json: bad id, package or version".into());
    }
    Ok(i)
}

/// Index entry for a `.wol` published at `<releases>/<package>-<version>/<package>-<version>.wol`;
/// its icon (if any) served at `<icons>/<id>.png`.
pub fn entry_of(
    wol: &[u8],
    releases: &str,
    abi: &str,
    icons: Option<&str>,
) -> Result<Entry, String> {
    let w = Wol::parse(wol)?;
    let i = info_of(&w.info)?;
    let file = format!("{}-{}", i.package, i.version);
    Ok(Entry {
        url: format!("{}/{file}/{file}.wol", releases.trim_end_matches('/')),
        sha256: sha256_hex(wol),
        size: wol.len() as u64,
        signed: w.sig.is_some(),
        icon: match (icons, &w.icon) {
            (Some(base), Some(_)) => format!("{}/{}.png", base.trim_end_matches('/'), i.id),
            _ => String::new(),
        },
        abi: abi.into(),
        id: i.id,
        name: i.name,
        summary: i.summary,
        version: i.version,
        package: i.package,
        categories: i.categories,
        capabilities: i.capabilities,
    })
}

impl Index {
    pub fn parse(bytes: &[u8]) -> Result<Index, String> {
        let i: Index = serde_json::from_slice(bytes).map_err(|e| format!("index: {e}"))?;
        if i.format != FORMAT {
            return Err(format!("index format {} (want {FORMAT})", i.format));
        }
        Ok(i)
    }

    /// One entry per id: a new build replaces the old one. Sorted by id.
    pub fn upsert(&mut self, e: Entry) {
        self.packages.retain(|p| p.id != e.id);
        self.packages.push(e);
        self.packages.sort_by(|a, b| a.id.cmp(&b.id));
    }

    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).unwrap_or_default();
        s.push('\n');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wol(id: &str, version: &str, signed: bool) -> Vec<u8> {
        let info = format!(
            r#"{{"id":"{id}","version":"{version}","name":"N","summary":"S","categories":["Work"],"capabilities":["files"],"package":"p{}","targets":[]}}"#,
            id.len()
        );
        Wol {
            info: info.into_bytes(),
            icon: None,
            payload: b"pkg".to_vec(),
            sig: signed.then(|| vec![0u8; 64]),
        }
        .to_bytes()
    }

    #[test]
    fn entry_from_a_wol() {
        let b = wol("org.wolos.A", "1.0", true);
        let e = entry_of(&b, "https://github.com/o/r/releases/download/", ABI, None).unwrap();
        assert_eq!(
            e.url,
            "https://github.com/o/r/releases/download/p11-1.0/p11-1.0.wol"
        );
        assert_eq!((e.size, e.signed), (b.len() as u64, true));
        assert_eq!(e.sha256, sha256_hex(&b));
        assert_eq!(e.categories, ["Work"]);
        assert_eq!((e.abi.as_str(), e.name.as_str()), (ABI, "N"));
        assert_eq!(e.icon, "");
        assert!(
            !Index {
                format: 1,
                packages: vec![e]
            }
            .to_json()
            .contains("icon")
        );
        let mut w = Wol::parse(&b).unwrap();
        w.icon = Some(b"PNG".to_vec());
        let e = entry_of(&w.to_bytes(), "https://x", ABI, Some("https://site/icons/")).unwrap();
        assert_eq!(e.icon, "https://site/icons/org.wolos.A.png");
        let e = entry_of(&b, "https://x", ABI, Some("https://site/icons")).unwrap();
        assert_eq!(e.icon, "", "no icon in the file, no icon URL");
    }

    #[test]
    fn upsert_replaces_by_id_and_sorts() {
        let mut idx = Index::default();
        for (id, v) in [("org.b.B", "1"), ("org.a.A", "1"), ("org.b.B", "2")] {
            idx.upsert(entry_of(&wol(id, v, false), "https://x", ABI, None).unwrap());
        }
        let got: Vec<(&str, &str)> = idx
            .packages
            .iter()
            .map(|p| (p.id.as_str(), p.version.as_str()))
            .collect();
        assert_eq!(got, [("org.a.A", "1"), ("org.b.B", "2")]);
        assert_eq!(Index::parse(idx.to_json().as_bytes()).unwrap(), idx);
    }

    #[test]
    fn rejects_table() {
        for bad in [
            r#"{"format":2,"packages":[]}"#,
            r#"{"packages":[]}"#,
            "not json",
        ] {
            assert!(Index::parse(bad.as_bytes()).is_err(), "{bad}");
        }
        for bad in [
            r#"{"id":"noreverse","version":"1","package":"p"}"#,
            r#"{"id":"a.b","version":"1-2","package":"p"}"#,
            r#"{"id":"a.b","version":"1","package":"-p"}"#,
            r#"{"id":"a.b","version":"1"}"#,
        ] {
            assert!(info_of(bad.as_bytes()).is_err(), "{bad}");
        }
    }
}

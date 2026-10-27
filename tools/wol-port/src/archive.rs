//! `.wol` file: ustar archive `Info.json`, `icon.png`?, `payload.pkg`, `SHA256`, `SHA256.sig`?
//! (WolOS ADR 0011, archicture/modules/packages.md).
use sha2::{Digest, Sha256};

pub const INFO: &str = "Info.json";
pub const ICON: &str = "icon.png";
pub const PAYLOAD: &str = "payload.pkg";
pub const MANIFEST: &str = "SHA256";
pub const SIG: &str = "SHA256.sig";

const BLOCK: usize = 512;

pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn octal(field: &mut [u8], v: u64) {
    let s = format!("{v:0w$o}", w = field.len() - 1);
    field[..s.len()].copy_from_slice(s.as_bytes());
    field[s.len()] = 0;
}

fn header(name: &str, size: usize) -> [u8; BLOCK] {
    let mut h = [0u8; BLOCK];
    h[..name.len()].copy_from_slice(name.as_bytes());
    octal(&mut h[100..108], 0o644);
    octal(&mut h[108..116], 0);
    octal(&mut h[116..124], 0);
    octal(&mut h[124..136], size as u64);
    octal(&mut h[136..148], 0);
    h[156] = b'0';
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    h[265..269].copy_from_slice(b"root");
    h[297..302].copy_from_slice(b"wheel");
    h[148..156].fill(b' ');
    let sum: u32 = h.iter().map(|b| u32::from(*b)).sum();
    let s = format!("{sum:06o}\0 ");
    h[148..156].copy_from_slice(s.as_bytes());
    h
}

/// Plain ustar: regular files, mode 0644, root:wheel, mtime 0.
pub fn tar(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, data) in members {
        out.extend_from_slice(&header(name, data.len()));
        out.extend_from_slice(data);
        out.resize(out.len().div_ceil(BLOCK) * BLOCK, 0);
    }
    out.resize(out.len() + 2 * BLOCK, 0);
    out
}

fn parse_octal(f: &[u8]) -> Option<u64> {
    let s = std::str::from_utf8(f).ok()?;
    let s = s.trim_matches(|c: char| c == '\0' || c == ' ');
    if s.is_empty() {
        return Some(0);
    }
    u64::from_str_radix(s, 8).ok()
}

/// Members of a ustar archive. Only regular files with plain names.
pub fn untar(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos + BLOCK <= bytes.len() {
        let h = &bytes[pos..pos + BLOCK];
        if h.iter().all(|b| *b == 0) {
            return Ok(out);
        }
        let mut check = h.to_vec();
        check[148..156].fill(b' ');
        let sum: u64 = check.iter().map(|b| u64::from(*b)).sum();
        if parse_octal(&h[148..156]) != Some(sum) {
            return Err("bad tar header checksum".into());
        }
        if h[257..262] != *b"ustar" {
            return Err("not a ustar archive".into());
        }
        if h[156] != b'0' && h[156] != 0 {
            return Err("only regular files are allowed".into());
        }
        if h[345] != 0 {
            return Err("member names must be plain".into());
        }
        let end = h[..100].iter().position(|b| *b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&h[..end])
            .map_err(|_| "member name is not UTF-8")?
            .to_string();
        let size = parse_octal(&h[124..136]).ok_or("bad member size")? as usize;
        let start = pos + BLOCK;
        if start + size > bytes.len() {
            return Err(format!("{name}: truncated"));
        }
        out.push((name, bytes[start..start + size].to_vec()));
        pos = start + size.div_ceil(BLOCK) * BLOCK;
    }
    Err("archive has no end marker".into())
}

/// Parsed `.wol` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wol {
    pub info: Vec<u8>,
    pub icon: Option<Vec<u8>>,
    pub payload: Vec<u8>,
    pub sig: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trust {
    /// Index of the key that verified the signature.
    Signed(usize),
    Unsigned,
}

impl Wol {
    fn hashed(&self) -> Vec<(&'static str, &[u8])> {
        let mut v = vec![(INFO, self.info.as_slice())];
        if let Some(i) = &self.icon {
            v.push((ICON, i.as_slice()));
        }
        v.push((PAYLOAD, self.payload.as_slice()));
        v
    }

    /// `SHA256 (<member>) = <hex>` lines, sorted by member name.
    pub fn manifest(&self) -> String {
        let mut lines: Vec<String> = self
            .hashed()
            .iter()
            .map(|(n, d)| format!("SHA256 ({n}) = {}\n", sha256_hex(d)))
            .collect();
        lines.sort();
        lines.concat()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let manifest = self.manifest();
        let mut m = self.hashed();
        m.push((MANIFEST, manifest.as_bytes()));
        if let Some(s) = &self.sig {
            m.push((SIG, s.as_slice()));
        }
        tar(&m)
    }

    /// Strict: exact names and order, no duplicates, the manifest matches every member.
    pub fn parse(bytes: &[u8]) -> Result<Wol, String> {
        let members = untar(bytes)?;
        let mut it = members.into_iter().peekable();
        let mut take = |name: &str, required: bool| -> Result<Option<Vec<u8>>, String> {
            match it.peek() {
                Some((n, _)) if n == name => Ok(it.next().map(|m| m.1)),
                _ if required => Err(format!("missing {name}")),
                _ => Ok(None),
            }
        };
        let info = take(INFO, true)?.unwrap_or_default();
        let icon = take(ICON, false)?;
        let payload = take(PAYLOAD, true)?.unwrap_or_default();
        let manifest = take(MANIFEST, true)?.unwrap_or_default();
        let sig = take(SIG, false)?;
        if let Some((n, _)) = it.next() {
            return Err(format!("unexpected member {n}"));
        }
        let w = Wol {
            info,
            icon,
            payload,
            sig,
        };
        if w.manifest().as_bytes() != manifest.as_slice() {
            return Err("SHA256 does not match the members".into());
        }
        if w.sig.as_ref().is_some_and(|s| s.len() != 64) {
            return Err("SHA256.sig must be 64 bytes".into());
        }
        Ok(w)
    }

    /// Signed by one of `keys`, unsigned, or an error (bad signature).
    pub fn verify(&self, keys: &[[u8; 32]]) -> Result<Trust, String> {
        let Some(sig) = &self.sig else {
            return Ok(Trust::Unsigned);
        };
        let m = self.manifest();
        keys.iter()
            .position(|k| crate::keys::verify(k, m.as_bytes(), sig))
            .map(Trust::Signed)
            .ok_or_else(|| "signature does not verify with any trusted key".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(icon: bool) -> Wol {
        Wol {
            info: br#"{"id":"org.wolos.Fixture"}"#.to_vec(),
            icon: icon.then(|| b"PNG".to_vec()),
            payload: vec![7u8; 1000],
            sig: None,
        }
    }

    #[test]
    fn tar_roundtrip_and_layout() {
        let t = tar(&[("a", b"hello"), ("b", b"")]);
        assert_eq!(t.len(), 512 * 2 + 512 + 1024);
        assert_eq!(&t[257..263], b"ustar\0");
        assert_eq!(
            untar(&t).unwrap(),
            vec![("a".into(), b"hello".to_vec()), ("b".into(), vec![])]
        );
    }

    #[test]
    fn manifest_lines_sorted_bsd_style() {
        let m = sample(true).manifest();
        let names: Vec<&str> = m
            .lines()
            .map(|l| l.split(['(', ')']).nth(1).unwrap())
            .collect();
        assert_eq!(names, ["Info.json", "icon.png", "payload.pkg"]);
        assert!(m.starts_with("SHA256 (Info.json) = "));
        assert_eq!(
            m.lines().next().unwrap().len(),
            "SHA256 (Info.json) = ".len() + 64
        );
    }

    #[test]
    fn parse_roundtrip_with_and_without_icon() {
        for icon in [false, true] {
            let w = sample(icon);
            assert_eq!(Wol::parse(&w.to_bytes()).unwrap(), w);
        }
    }

    #[test]
    fn parse_rejects_table() {
        let w = sample(true);
        let man = w.manifest();
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("no end", tar(&[(INFO, b"{}")])[..1024].to_vec()),
            ("missing info", tar(&[(PAYLOAD, b"x"), (MANIFEST, b"")])),
            (
                "wrong order",
                tar(&[
                    (PAYLOAD, &w.payload),
                    (INFO, &w.info),
                    (MANIFEST, man.as_bytes()),
                ]),
            ),
            (
                "extra member",
                tar(&[
                    (INFO, &w.info),
                    (ICON, b"PNG"),
                    (PAYLOAD, &w.payload),
                    (MANIFEST, man.as_bytes()),
                    ("evil.sh", b"rm -rf /"),
                ]),
            ),
            (
                "tampered payload",
                tar(&[
                    (INFO, &w.info),
                    (ICON, b"PNG"),
                    (PAYLOAD, b"other"),
                    (MANIFEST, man.as_bytes()),
                ]),
            ),
            (
                "short sig",
                tar(&[
                    (INFO, &w.info),
                    (ICON, b"PNG"),
                    (PAYLOAD, &w.payload),
                    (MANIFEST, man.as_bytes()),
                    (SIG, b"short"),
                ]),
            ),
        ];
        for (what, bytes) in cases {
            assert!(Wol::parse(&bytes).is_err(), "{what}");
        }
        let mut bad = w.to_bytes();
        bad[0] = b'X';
        assert!(Wol::parse(&bad).is_err(), "checksum");
    }

    #[test]
    fn verify_good_bad_missing() {
        let seed = [1u8; 32];
        let other = [2u8; 32];
        let pk = crate::keys::public_of(&seed);
        let opk = crate::keys::public_of(&other);
        let mut w = sample(false);
        assert_eq!(w.verify(&[pk]), Ok(Trust::Unsigned));
        w.sig = Some(crate::keys::sign(&seed, w.manifest().as_bytes()).to_vec());
        let w = Wol::parse(&w.to_bytes()).unwrap();
        assert_eq!(w.verify(&[opk, pk]), Ok(Trust::Signed(1)));
        assert!(w.verify(&[opk]).is_err());
        assert!(w.verify(&[]).is_err());
    }
}

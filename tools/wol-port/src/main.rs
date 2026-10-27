//! `wol-port` CLI: thin wrapper over the library (file and process I/O only).
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use wol_port::{archive, depmap, detect, index, keys, recipe::Recipe, script};

const USAGE: &str = "usage:
  wol-port new <git-url> [--tag T] [--dir recipes]   draft recipes/<name>/recipe.toml
  wol-port check <recipe-dir>...                     validate recipes
  wol-port plan <recipe-dir>                         FreeBSD build script on stdout
  wol-port info <recipe-dir>                         Info.json on stdout
  wol-port pack --info F --payload F [--icon F] -o F build a .wol
  wol-port sign <file.wol> <key.pem>                 add SHA256.sig (Ed25519)
  wol-port verify <file.wol> [pub.pem...]            check hashes and signature
  wol-port index --index F --releases URL [--icons URL] [--abi A] [--key key.pem] <file.wol>...
  wol-port map <pc|prog|cmake|crate|deb> <name>...   FreeBSD pkg for upstream names";

fn read(p: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn write(p: &Path, data: &[u8]) -> Result<(), String> {
    std::fs::write(p, data).map_err(|e| format!("{}: {e}", p.display()))
}

fn text(p: &Path) -> Result<String, String> {
    String::from_utf8(read(p)?).map_err(|_| format!("{}: not UTF-8", p.display()))
}

/// `--flag value` pairs and positionals.
fn split(args: &[String]) -> (Vec<(String, String)>, Vec<String>) {
    let (mut flags, mut pos) = (Vec::new(), Vec::new());
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a
            .strip_prefix("--")
            .or(a.strip_prefix('-').filter(|s| s.len() == 1))
        {
            Some(f) => flags.push((f.to_string(), it.next().cloned().unwrap_or_default())),
            None => pos.push(a.clone()),
        }
    }
    (flags, pos)
}

fn flag<'a>(flags: &'a [(String, String)], name: &str) -> Option<&'a str> {
    flags
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn need<'a>(flags: &'a [(String, String)], name: &str) -> Result<&'a str, String> {
    flag(flags, name).ok_or_else(|| format!("missing --{name}"))
}

fn run(args: &[String]) -> Result<String, String> {
    let Some((cmd, rest)) = args.split_first() else {
        return Err(USAGE.into());
    };
    let (flags, pos) = split(rest);
    match cmd.as_str() {
        "new" => {
            let url = pos.first().ok_or(USAGE)?;
            let dir = PathBuf::from(flag(&flags, "dir").unwrap_or("recipes"));
            let draft = detect::new_recipe(url, flag(&flags, "tag"))?;
            let out = dir.join(&draft.package);
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let file = out.join(wol_port::recipe::FILE);
            if file.exists() {
                return Err(format!("{} exists", file.display()));
            }
            write(&file, draft.text.as_bytes())?;
            Ok(format!("wrote {}\n{}", file.display(), draft.notes))
        }
        "check" => {
            for d in &pos {
                Recipe::load(Path::new(d))?;
            }
            Ok(format!("{} recipe(s) ok\n", pos.len()))
        }
        "plan" => {
            let d = Path::new(pos.first().ok_or(USAGE)?);
            Ok(script::plan(&Recipe::load(d)?, d))
        }
        "info" => Ok(Recipe::load(Path::new(pos.first().ok_or(USAGE)?))?.info_json()),
        "pack" => {
            let info = read(Path::new(need(&flags, "info")?))?;
            index::info_of(&info)?;
            let icon = flag(&flags, "icon")
                .map(|p| read(Path::new(p)))
                .transpose()?;
            let w = archive::Wol {
                info,
                icon,
                payload: read(Path::new(need(&flags, "payload")?))?,
                sig: None,
            };
            let out = Path::new(need(&flags, "o")?);
            write(out, &w.to_bytes())?;
            Ok(format!("packed {}\n", out.display()))
        }
        "sign" => {
            let [file, key] = pos.as_slice() else {
                return Err(USAGE.into());
            };
            let mut w = archive::Wol::parse(&read(Path::new(file))?)?;
            let seed = keys::seed_from_pem(&text(Path::new(key))?)?;
            w.sig = Some(keys::sign(&seed, w.manifest().as_bytes()).to_vec());
            write(Path::new(file), &w.to_bytes())?;
            Ok(format!("signed {file}\n"))
        }
        "verify" => {
            let file = pos.first().ok_or(USAGE)?;
            let w = archive::Wol::parse(&read(Path::new(file))?)?;
            let info = index::info_of(&w.info)?;
            let pems = &pos[1..];
            let keys = pems
                .iter()
                .map(|p| keys::public_from_pem(&text(Path::new(p))?))
                .collect::<Result<Vec<_>, _>>()?;
            let who = match w.verify(&keys)? {
                archive::Trust::Signed(i) => format!("signed by {}", pems[i]),
                archive::Trust::Unsigned => "unsigned".into(),
            };
            Ok(format!(
                "{file}: {} {} ({}): {who}\n",
                info.package, info.version, info.id
            ))
        }
        "index" => {
            let path = Path::new(need(&flags, "index")?);
            let mut idx = match std::fs::read(path) {
                Ok(b) => index::Index::parse(&b)?,
                Err(_) => index::Index::default(),
            };
            let releases = need(&flags, "releases")?;
            let abi = flag(&flags, "abi").unwrap_or(index::ABI);
            for f in &pos {
                let bytes = read(Path::new(f))?;
                idx.upsert(index::entry_of(
                    &bytes,
                    releases,
                    abi,
                    flag(&flags, "icons"),
                )?);
            }
            let json = idx.to_json();
            write(path, json.as_bytes())?;
            let mut msg = format!("{}: {} package(s)\n", path.display(), idx.packages.len());
            if let Some(k) = flag(&flags, "key") {
                let seed = keys::seed_from_pem(&text(Path::new(k))?)?;
                let sig_path = PathBuf::from(format!("{}.sig", path.display()));
                write(&sig_path, &keys::sign(&seed, json.as_bytes()))?;
                msg.push_str(&format!("signed {}\n", sig_path.display()));
            }
            Ok(msg)
        }
        "map" => {
            let (kind, names) = pos.split_first().ok_or(USAGE)?;
            let kind = depmap::Kind::parse(kind).ok_or(USAGE)?;
            Ok(names
                .iter()
                .map(|n| format!("{n}\t{}\n", depmap::map(kind, n).describe()))
                .collect())
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("wol-port: {e}");
            ExitCode::FAILURE
        }
    }
}

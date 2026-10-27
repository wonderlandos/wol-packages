# wol-packages

Open-source desktop apps built natively for WolOS (FreeBSD 15, amd64) and published as `.wol` files.
A `.wol` is one file you can double-click: app info, an icon and a FreeBSD `pkg` package, with a signed
SHA-256 manifest. Dependencies come from FreeBSD packages; no Linux compatibility layer.
Format: WolOS `archicture/modules/packages.md` (ADR 0011).

Apps in the [index](https://wonderlandos.github.io/wol-packages/index.json); files under Releases.

## Add an app

```sh
cargo build --release --manifest-path tools/wol-port/Cargo.toml
alias wol-port=tools/wol-port/target/release/wol-port
wol-port new https://github.com/owner/app.git      # drafts recipes/app/recipe.toml
$EDITOR recipes/app/recipe.toml                     # fill what the notes ask for
wol-port check recipes/app
```

Open a pull request. CI builds every changed recipe in a FreeBSD 15.1 VM, installs and removes the
package once, and uploads the `.wol`. On `main` it publishes the Release and the index.

| `recipe.toml` | Meaning |
|---|---|
| `id`, `name`, `summary`, `icon`, `categories`, `capabilities` | what the App Store shows |
| `package`, `version` | pkg name and version |
| `entry` | the app's executable, relative to `/usr/local` |
| `[source]` `git`, `tag` | where the code is |
| `[build]` `system` | `meson`, `cmake`, `cargo`, `make` or `autotools`; `options`, `env`, `make` (`gmake` only if needed) |
| `[deps]` `build`, `run` | FreeBSD pkg names; libraries the binaries link are added automatically |
| `patches/*.patch` | applied with `patch -p1` in name order |

`wol-port map pc gtk4 libsystemd` shows how upstream names map to FreeBSD packages.

## Commands

| Command | Does |
|---|---|
| `wol-port new <git-url> [--tag T] [--dir recipes]` | draft a recipe from a source tree |
| `wol-port check <recipe-dir>...` | validate recipes |
| `wol-port plan <recipe-dir>` | the FreeBSD build script |
| `wol-port info <recipe-dir>` | the app's `Info.json` |
| `wol-port pack --info F --payload F [--icon F] -o F` | build a `.wol` |
| `wol-port sign <file.wol> <key.pem>` | add the Ed25519 signature |
| `wol-port verify <file.wol> [pub.pem...]` | check hashes and signature |
| `wol-port index --index F --releases URL [--icons URL] [--key key.pem] <file.wol>...` | update `index.json` (and `index.json.sig`) |

## Signing

CI signs `.wol` files and the index when the `WOL_SIGNING_KEY` Actions secret exists; without it the
files are published unsigned and WolOS asks for the administrator password on every install.

```sh
openssl genpkey -algorithm ed25519 -out wol-packages.key
openssl pkey -in wol-packages.key -pubout -out wol-packages.pub   # ships in WolOS
gh secret set WOL_SIGNING_KEY < wol-packages.key
```

Never commit the private key.

## License

Tooling and recipes: BSD-2-Clause. Each app keeps its own license (`license` in its recipe).

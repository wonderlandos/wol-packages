//! wol-port: draft recipes from source trees, emit FreeBSD build scripts, pack, sign and index
//! `.wol` packages (WolOS ADR 0011).
pub mod archive;
pub mod depmap;
pub mod detect;
pub mod index;
pub mod keys;
pub mod recipe;
pub mod script;

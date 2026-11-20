//! The files of a corpus: `manifest.json`, its schema, and the files
//! `<id>.xdr` and `<id>.json` of each source in `sources/`.

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const SOURCES: &str = "sources";

pub fn xdr(corpus: &Path, id: &str) -> PathBuf {
    corpus.join(SOURCES).join(format!("{id}.xdr"))
}

pub fn json(corpus: &Path, id: &str) -> PathBuf {
    corpus.join(SOURCES).join(format!("{id}.json"))
}

pub fn manifest(corpus: &Path) -> PathBuf {
    corpus.join("manifest.json")
}

pub const SCHEMA: &str = "manifest.schema.json";

pub fn schema(corpus: &Path) -> PathBuf {
    corpus.join(SCHEMA)
}

pub fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("read {}", path.display()))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&read(path)?)
        .with_context(|| format!("parse {}", path.display()))
}

pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .with_context(|| format!("create {}", dir.display()))?;
    }
    fs::write(path, bytes).with_context(|| format!("write {}", path.display()))
}

/// Writes pretty JSON with a final newline.
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    write(path, text.as_bytes())
}

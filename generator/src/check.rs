//! `check`: verifies the schema file and every source and entry of the
//! manifest, and warns about a file that no source names.

use crate::{
    codec, files,
    manifest::{Entry, Manifest, Origin, SorobanValidity, Source},
};
use anyhow::{Context, Result, anyhow, bail, ensure};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};
use stellar_xdr::{LedgerCloseMeta, Type};

/// A source as its two files hold it: its bytes and its decoded value.
struct Stored {
    bytes: Vec<u8>,
    decoded: Value,
}

/// Checks the corpus in `corpus` and prints one line for each check. Fails
/// when any check fails. A file that no source names gives a warning alone.
pub fn run(corpus: &Path) -> Result<()> {
    let manifest: Manifest = files::read_json(&files::manifest(corpus))?;
    let unnamed = unnamed_files(corpus, &manifest);
    let mut lines = Lines {
        width: [files::SCHEMA, files::SOURCES]
            .into_iter()
            .chain(manifest.sources.iter().map(|source| source.id.as_str()))
            .chain(manifest.entries.iter().map(|entry| entry.id.as_str()))
            .chain(unnamed.iter().flatten().map(String::as_str))
            .map(str::len)
            .max()
            .unwrap_or_default(),
        passed: 0,
        failed: 0,
        warned: 0,
    };
    let mut sources = HashMap::new();
    let mut ids = HashSet::new();

    let schema: Value = files::read_json(&files::schema(corpus))?;
    let current = serde_json::to_value(Manifest::json_schema())?;
    let same = if schema == current {
        Ok(())
    } else {
        Err(anyhow!(
            "the file differs from the schema of the Rust types of the \
             manifest: run generate to write it again"
        ))
    };
    lines.report("schema", files::SCHEMA, same);

    for source in &manifest.sources {
        let result = unique(&mut ids, &source.id)
            .and_then(|()| read(corpus, &source.id))
            .and_then(|stored| {
                let checked = check_source(source, &stored);
                sources.insert(source.id.as_str(), (source, stored));
                checked
            });
        lines.report("source", &source.id, result);
    }
    for entry in &manifest.entries {
        let name = &entry.location.source;
        let source = sources.get(name.as_str()).with_context(|| {
            if manifest.sources.iter().any(|source| &source.id == name) {
                format!(
                    "the source {name} failed, so the entry has no bytes \
                     to check"
                )
            } else {
                format!("the manifest has no source {name}")
            }
        });
        let result = unique(&mut ids, &entry.id)
            .and(source)
            .and_then(|(source, stored)| check_entry(entry, source, stored));
        lines.report("entry", &entry.id, result);
    }
    match unnamed {
        Ok(names) => {
            for name in names {
                lines.warn(
                    "file",
                    &name,
                    "no source of the manifest has this file: delete it",
                );
            }
        }
        Err(error) => lines.report("files", files::SOURCES, Err(error)),
    }
    lines.summary()
}

/// The report of `check`: one line for each check, padded to `width`, and
/// the count of each kind of line.
struct Lines {
    width: usize,
    passed: usize,
    failed: usize,
    warned: usize,
}

impl Lines {
    /// Prints one line for the check of `id`.
    fn report(&mut self, kind: &str, id: &str, result: Result<()>) {
        let width = self.width;
        match result {
            Ok(()) => {
                self.passed += 1;
                println!("pass  {kind:6}  {id}");
            }
            Err(error) => {
                self.failed += 1;
                println!("fail  {kind:6}  {id:width$}  {error:#}");
            }
        }
    }

    /// Prints one line that warns about `id`, which does not fail the check.
    fn warn(&mut self, kind: &str, id: &str, text: &str) {
        let width = self.width;
        self.warned += 1;
        println!("warn  {kind:6}  {id:width$}  {text}");
    }

    /// Prints the count of each kind of line. Fails when any line failed.
    fn summary(&self) -> Result<()> {
        let total = self.passed + self.failed + self.warned;
        let parts: Vec<String> = [
            (self.passed, "pass"),
            (self.failed, "fail"),
            (self.warned, "warn"),
        ]
        .into_iter()
        .filter(|&(count, _)| count > 0)
        .map(|(count, label)| format!("{count} {label}"))
        .collect();
        println!("\n{total} lines: {}", parts.join(", "));
        ensure!(self.failed == 0, "{} of {total} lines failed", self.failed);
        Ok(())
    }
}

/// The files in `sources/` that no source of the manifest names, as sorted
/// paths inside the corpus. A source whose `id` changed leaves such files.
fn unnamed_files(corpus: &Path, manifest: &Manifest) -> Result<Vec<String>> {
    let named: HashSet<_> = manifest
        .sources
        .iter()
        .flat_map(|source| {
            [
                files::xdr(corpus, &source.id),
                files::json(corpus, &source.id),
            ]
        })
        .collect();
    let dir = corpus.join(files::SOURCES);
    let mut unnamed = Vec::new();
    for file in
        fs::read_dir(&dir).with_context(|| format!("list {}", dir.display()))?
    {
        let path = file?.path();
        if !named.contains(&path) {
            unnamed.push(path.strip_prefix(corpus)?.display().to_string());
        }
    }
    unnamed.sort();
    Ok(unnamed)
}

/// Adds `id` to `ids`. Fails when `id` is empty, holds a character other
/// than a-z, 0-9 and -, or is already in `ids`.
// NOTE: sources and entries share one set of ids, because the runners and
// their files of known verdicts name an item by its id alone. The id of a
// source also names its files.
fn unique<'a>(ids: &mut HashSet<&'a str>, id: &'a str) -> Result<()> {
    let plain =
        |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    ensure!(
        !id.is_empty() && id.chars().all(plain),
        "an id holds lower-case letters, digits and hyphens alone, and this \
         id is empty or holds another character"
    );
    ensure!(ids.insert(id), "another source or entry has this id");
    Ok(())
}

fn read(corpus: &Path, id: &str) -> Result<Stored> {
    Ok(Stored {
        bytes: files::read(&files::xdr(corpus, id))?,
        decoded: files::read_json(&files::json(corpus, id))?,
    })
}

/// A source passes when its bytes decode to its JSON file and the JSON
/// encodes to the same bytes. A source of stellar-core must also be a
/// `LedgerCloseMeta` closed under `ledger_protocol`.
fn check_source(source: &Source, stored: &Stored) -> Result<()> {
    let variant = codec::variant(&source.xdr_type)?;
    let from_bytes = codec::from_bytes(variant, &stored.bytes)?;
    let from_json = codec::from_json(variant, &stored.decoded)?;
    ensure!(
        codec::to_json(&from_bytes)? == stored.decoded,
        "the bytes decode to a value other than the JSON file"
    );
    ensure!(
        codec::to_bytes(&from_json)? == stored.bytes,
        "the JSON file encodes to other bytes"
    );
    if let Origin::StellarCore {
        ledger_protocol, ..
    } = source.origin
    {
        let Type::LedgerCloseMeta(ledger) = &from_bytes else {
            bail!(
                "the origin stellar_core needs a LedgerCloseMeta, and the \
                 source is a {}",
                from_bytes.variant().name()
            );
        };
        let header = match ledger.as_ref() {
            LedgerCloseMeta::V0(meta) => &meta.ledger_header,
            LedgerCloseMeta::V1(meta) => &meta.ledger_header,
            LedgerCloseMeta::V2(meta) => &meta.ledger_header,
        };
        let version = header.header.ledger_version;
        ensure!(
            version == ledger_protocol,
            "the ledger closed under protocol {version}, and ledger_protocol \
             states {ledger_protocol}"
        );
    }
    Ok(())
}

/// An entry passes when its byte range lies inside the source and decodes
/// to the value at its JSON Pointer. For a source of stellar-core, its
/// `since_protocol` must not be above `ledger_protocol`. Its
/// `soroban_validity` must be the answer of the Soroban host.
fn check_entry(entry: &Entry, source: &Source, stored: &Stored) -> Result<()> {
    let place = &entry.location;
    let range = stored
        .bytes
        .get(place.byte_range.offset..)
        .and_then(|rest| rest.get(..place.byte_range.length))
        .with_context(|| {
            format!(
                "the byte range ends at byte {}, after the {} bytes of the \
                 source",
                place
                    .byte_range
                    .offset
                    .saturating_add(place.byte_range.length),
                stored.bytes.len()
            )
        })?;
    let at_path =
        stored
            .decoded
            .pointer(&place.json_pointer)
            .with_context(|| {
                format!(
                    "the JSON Pointer {} names no value in the source",
                    place.json_pointer
                )
            })?;
    let shape = codec::from_bytes(codec::variant(&entry.xdr_type)?, range)?;
    ensure!(
        &codec::to_json(&shape)? == at_path,
        "the byte range decodes to a value other than the value at the JSON \
         Pointer"
    );
    if let Origin::StellarCore {
        ledger_protocol, ..
    } = source.origin
    {
        ensure!(
            entry.since_protocol <= ledger_protocol,
            "since_protocol is {}, above the protocol {ledger_protocol} of \
             the ledger",
            entry.since_protocol
        );
    }
    let answer = codec::soroban_validity(&shape);
    ensure!(
        answer == entry.soroban_validity,
        "soroban_validity states {}, and the Soroban host answers {}",
        validity_name(entry.soroban_validity)?,
        validity_name(answer)?
    );
    Ok(())
}

/// The name of `validity` in the manifest, such as `not_applicable`.
fn validity_name(validity: SorobanValidity) -> Result<String> {
    match serde_json::to_value(validity)? {
        Value::String(name) => Ok(name),
        other => bail!("soroban_validity is no name: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    const MEMO_SOURCE: &str = "ledger-memo-text";

    /// A copy of the corpus of the repository, which a test may change.
    fn corpus() -> TempDir {
        let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("../corpus");
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(files::SOURCES)).unwrap();
        for part in ["", files::SOURCES] {
            for file in fs::read_dir(from.join(part)).unwrap() {
                let file = file.unwrap().path();
                if file.is_file() {
                    let name = file.file_name().unwrap();
                    fs::copy(&file, dir.path().join(part).join(name)).unwrap();
                }
            }
        }
        dir
    }

    /// Whether `check` passes after `change` edits the manifest.
    fn passes_with(change: impl FnOnce(&mut Value)) -> bool {
        let dir = corpus();
        let path = files::manifest(dir.path());
        let mut manifest: Value = files::read_json(&path).unwrap();
        change(&mut manifest);
        files::write_json(&path, &manifest).unwrap();
        run(dir.path()).is_ok()
    }

    fn memo(manifest: &mut Value) -> &mut Value {
        manifest["entries"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["id"] == "memo-text-non-utf8")
            .unwrap()
    }

    #[test]
    fn passes_the_corpus_of_the_repository() {
        assert!(passes_with(|_| {}));
    }

    #[test]
    fn fails_on_a_schema_file_that_differs_from_the_types() {
        let dir = corpus();
        let path = files::schema(dir.path());
        let mut schema: Value = files::read_json(&path).unwrap();
        schema["title"] = json!("Another");
        files::write_json(&path, &schema).unwrap();
        assert!(run(dir.path()).is_err());
    }

    #[test]
    fn fails_on_an_id_that_names_another_directory() {
        assert!(!passes_with(|manifest| {
            memo(manifest)["id"] = json!("../memo");
        }));
    }

    #[test]
    fn fails_on_an_id_that_repeats() {
        assert!(!passes_with(|manifest| {
            memo(manifest)["id"] = json!(MEMO_SOURCE);
        }));
    }

    #[test]
    fn fails_on_a_json_file_that_differs_from_the_bytes() {
        let dir = corpus();
        let path = files::json(dir.path(), MEMO_SOURCE);
        let mut json: Value = files::read_json(&path).unwrap();
        json["v2"]["ledger_header"]["header"]["ledger_seq"] = json!(1);
        files::write_json(&path, &json).unwrap();
        assert!(run(dir.path()).is_err());
    }

    #[test]
    fn fails_on_a_ledger_protocol_other_than_the_header() {
        assert!(!passes_with(|manifest| {
            let origin = &mut manifest["sources"][0]["origin"];
            origin["stellar_core"]["ledger_protocol"] = json!(29);
        }));
    }

    #[test]
    fn fails_on_an_unknown_source() {
        assert!(!passes_with(|manifest| {
            memo(manifest)["location"]["source"] = json!("no-such-source");
        }));
    }

    #[test]
    fn fails_on_a_byte_range_outside_the_source() {
        assert!(!passes_with(|manifest| {
            memo(manifest)["location"]["byte_range"]["length"] =
                json!(1_000_000);
        }));
    }

    #[test]
    fn fails_on_a_path_to_another_value() {
        assert!(!passes_with(|manifest| {
            memo(manifest)["location"]["json_pointer"] = json!("/v2");
        }));
    }

    #[test]
    fn fails_on_a_shape_newer_than_its_ledger() {
        assert!(!passes_with(|manifest| {
            memo(manifest)["since_protocol"] = json!(31);
        }));
    }

    #[test]
    fn fails_on_a_soroban_validity_other_than_the_answer_of_the_host() {
        assert!(!passes_with(|manifest| {
            memo(manifest)["soroban_validity"] = json!("valid");
        }));
    }
}

//! `generate`: produces the corpus from the scenarios.

use crate::{
    check, codec, files,
    manifest::{self, ByteRange, Entry, Location, Manifest, Origin},
    network::Network,
    node::{self, NodeConfig},
    scenarios::{self, Scenario, Source},
};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::path::Path;
use stellar_xdr::Type;

/// Writes the corpus and then runs `check` on it.
pub fn run(corpus: &Path, config: &NodeConfig) -> Result<()> {
    let mut manifest = Manifest {
        schema: files::SCHEMA.to_string(),
        network_passphrase: config.passphrase.clone(),
        sources: Vec::new(),
        entries: Vec::new(),
    };
    node::require_version(config)?;
    let core = Origin::StellarCore {
        version: config.version.to_string(),
        ledger_protocol: config.protocol,
    };
    for (name, scenario) in scenarios::ALL {
        produce(corpus, config, &core, scenario, &mut manifest)
            .with_context(|| format!("scenario {name}"))?;
    }
    files::write_json(&files::manifest(corpus), &manifest)?;
    files::write_json(&files::schema(corpus), &Manifest::json_schema())?;
    check::run(corpus)
}

/// Runs `scenario`, writes the files of its source, and adds the source and
/// its entries to `manifest`.
fn produce(
    corpus: &Path,
    config: &NodeConfig,
    core: &Origin,
    scenario: Scenario,
    manifest: &mut Manifest,
) -> Result<()> {
    let (source, origin) = match scenario {
        Scenario::Ledger(run) => {
            // NOTE: a new network for each scenario keeps the bytes
            // of a source independent of the order of the scenarios.
            let source = run(&mut Network::start(config)?)?;
            let source = Source {
                id: source.id,
                shapes: source.shapes,
                value: Type::LedgerCloseMeta(Box::new(source.value)),
            };
            (source, core.clone())
        }
        Scenario::Synthetic(build) => {
            (build(config.network_id())?, Origin::Synthetic)
        }
    };
    let value = source.value;
    let bytes = codec::to_bytes(&value)?;
    let json = codec::to_json(&value)?;
    files::write(&files::xdr(corpus, source.id), &bytes)?;
    files::write_json(&files::json(corpus, source.id), &json)?;

    for shape in source.shapes {
        let id = shape.id;
        let entry = entry(source.id, shape, &bytes, &json)
            .with_context(|| format!("entry {id}"))?;
        manifest.entries.push(entry);
    }
    manifest.sources.push(manifest::Source {
        id: source.id.to_string(),
        xdr_type: value.variant().name().to_string(),
        origin,
    });
    Ok(())
}

/// The manifest entry of `shape` in the source `source`, whose files hold
/// `bytes` and `json`.
fn entry(
    source: &str,
    shape: scenarios::Shape,
    bytes: &[u8],
    json: &Value,
) -> Result<Entry> {
    ensure!(
        shape.caps.is_sorted_by(|a, b| a < b),
        "list the caps in ascending order"
    );
    let value = json.pointer(&shape.json_pointer).with_context(|| {
        format!(
            "the JSON Pointer {} names no value in the source",
            shape.json_pointer
        )
    })?;
    let decoded = codec::from_json(shape.xdr_type, value)?;
    let cut = codec::to_bytes(&decoded)?;
    let offset = find(bytes, &cut, json, value, &shape.json_pointer)?;
    Ok(Entry {
        id: shape.id.to_string(),
        description: shape.description.to_string(),
        caps: shape
            .caps
            .iter()
            .map(|cap| format!("CAP-{cap:04}"))
            .collect(),
        since_protocol: shape.since_protocol,
        xdr_type: shape.xdr_type.name().to_string(),
        soroban_validity: codec::soroban_validity(&decoded),
        location: Location {
            source: source.to_string(),
            json_pointer: shape.json_pointer,
            byte_range: ByteRange {
                offset,
                length: cut.len(),
            },
        },
        sightings: shape.sightings.iter().map(|s| s.to_string()).collect(),
    })
}

/// The offset in `source` of the bytes `shape` of the value at `at`. `value`
/// may stand at several places of `json`, and then `shape` stands at as many
/// offsets. The place `at` takes the offset of the same rank. Fails when `at`
/// is not a place of `value`, or when offsets and places differ in count.
// NOTE: serde_json keeps the fields of an object in the order of the XDR,
// through its feature `preserve_order`. So the places of a value in the JSON
// come in the order of their bytes.
// TODO: compute the offset from the path. The rank fails when bytes
// that are not the value match too, for example inside the bytes of a Wasm.
fn find(
    source: &[u8],
    shape: &[u8],
    json: &Value,
    value: &Value,
    at: &str,
) -> Result<usize> {
    // NOTE: XDR aligns every value to 4 bytes, so the search steps by 4.
    let offsets: Vec<usize> = (0..=source.len().saturating_sub(shape.len()))
        .step_by(4)
        .filter(|&offset| source[offset..].starts_with(shape))
        .collect();
    let mut places = Vec::new();
    codec::pointers_to(json, value, String::new(), &mut places);
    ensure!(
        offsets.len() == places.len(),
        "the bytes of the shape stand at {} offsets {offsets:?}, and the value \
         at {} places {places:?}, so the offset is unclear",
        offsets.len(),
        places.len()
    );
    let rank = places.iter().position(|place| place == at);
    let rank = rank.with_context(|| {
        format!("the search found the value at {places:?} and not at {at}")
    })?;
    Ok(offsets[rank])
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::ScVal;

    /// The offset of `shape` at `at` in a vector of Soroban values.
    fn offset(items: Vec<ScVal>, shape: ScVal, at: &str) -> Result<usize> {
        let source =
            Type::ScVal(Box::new(ScVal::Vec(Some(items.try_into().unwrap()))));
        let shape = Type::ScVal(Box::new(shape));
        find(
            &codec::to_bytes(&source)?,
            &codec::to_bytes(&shape)?,
            &codec::to_json(&source)?,
            &codec::to_json(&shape)?,
            at,
        )
    }

    #[test]
    fn find_tells_two_places_of_one_value_apart() {
        let items = || vec![ScVal::U32(7), ScVal::U32(1), ScVal::U32(7)];
        // NOTE: the vector starts with 12 bytes: the kind of the value,
        // the flag of the optional vector and the count of its items.
        // A u32 value takes 8 bytes.
        assert_eq!(offset(items(), ScVal::U32(7), "/vec/0").unwrap(), 12);
        assert_eq!(offset(items(), ScVal::U32(7), "/vec/2").unwrap(), 28);
    }

    #[test]
    fn find_fails_when_other_bytes_match_too() {
        // NOTE: the bytes value holds the 8 bytes of the u32 value 7. So the
        // bytes of the shape stand at two offsets, and the value at one place.
        let bytes =
            ScVal::Bytes(vec![0, 0, 0, 3, 0, 0, 0, 7].try_into().unwrap());
        let items = vec![bytes, ScVal::U32(7)];
        assert!(offset(items, ScVal::U32(7), "/vec/1").is_err());
    }
}

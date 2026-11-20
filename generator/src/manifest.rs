//! The manifest schema: sources and the entries that point into them.

use schemars::{
    JsonSchema, Schema, generate::SchemaSettings, transform::RecursiveTransform,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The whole manifest. `generate` writes it, and `check` reads it.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The file name of the JSON Schema of this manifest.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// The passphrase of the network that signed and closed the ledgers.
    pub network_passphrase: String,
    /// The sources: one XDR value each.
    pub sources: Vec<Source>,
    /// The entries: one rare shape each.
    pub entries: Vec<Entry>,
}

/// One XDR value in the files `<id>.xdr` and `<id>.json`.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// The name of the source and of its two files.
    #[schemars(regex(pattern = r"^[a-z0-9-]+$"))]
    pub id: String,
    /// The XDR type of the value.
    pub xdr_type: String,
    /// Where the bytes come from.
    pub origin: Origin,
}

/// Where the bytes of a source come from.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Origin {
    /// A local stellar-core closed the ledger.
    StellarCore {
        /// The release of stellar-core.
        version: String,
        /// The protocol under which the ledger closed.
        ledger_protocol: u32,
    },
    /// The generator built the bytes, and stellar-core did not emit them.
    Synthetic,
}

/// One rare shape: a place in a source.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The name of the entry.
    #[schemars(regex(pattern = r"^[a-z0-9-]+$"))]
    pub id: String,
    /// The fact that a reader of the shape must keep.
    pub description: String,
    /// The CAPs that introduced the shape, ascending. The list is empty for
    /// a shape that the network had at launch.
    #[schemars(inner(regex(pattern = r"^CAP-[0-9]{4}$")))]
    pub caps: Vec<String>,
    /// The first protocol under which the network can emit the shape, 0 for
    /// a shape that the network had at launch.
    pub since_protocol: u32,
    /// The XDR type of the shape.
    pub xdr_type: String,
    /// What the Soroban host answers about the shape.
    pub soroban_validity: SorobanValidity,
    /// The place of the shape in a source.
    pub location: Location,
    /// The upstream issues and pull requests where the shape broke a reader.
    pub sightings: Vec<String>,
}

/// What the Soroban host, the engine that stellar-core runs, answers
/// about a value.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SorobanValidity {
    /// The host accepts the value.
    Valid,
    /// The host refuses the value. stellar-core can still write it to a
    /// ledger, for example as the argument of a call that fails.
    Invalid,
    /// The value is not a Soroban value, so the host has no answer.
    NotApplicable,
}

/// The place of a shape, written in two ways.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// The `id` of the source.
    pub source: String,
    /// A JSON Pointer into the decoded source, empty for the whole value.
    pub json_pointer: String,
    /// The same place in the bytes of the source.
    pub byte_range: ByteRange,
}

/// A range of the bytes of a source: `length` bytes from `offset`.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ByteRange {
    pub offset: usize,
    pub length: usize,
}

impl Manifest {
    /// The JSON Schema of the manifest, derived from these types.
    pub fn json_schema() -> Schema {
        // NOTE: schemars marks each integer with the name of its Rust
        // type, such as `"format": "uint32"`. JSON Schema defines no such
        // format, and Ajv in strict mode, which the JS runner uses, refuses
        // the schema over it. The type `integer` and the minimum 0 stay.
        let no_format = RecursiveTransform(|schema: &mut Schema| {
            schema.remove("format");
        });
        // NOTE: schemars keeps the line breaks of a doc comment, and an
        // editor shows a description as one paragraph.
        let one_line = RecursiveTransform(|schema: &mut Schema| {
            if let Some(Value::String(text)) = schema.get_mut("description") {
                *text = text.replace('\n', " ");
            }
        });
        SchemaSettings::default()
            .with_transform(no_format)
            .with_transform(one_line)
            .into_generator()
            .into_root_schema_for::<Manifest>()
    }
}

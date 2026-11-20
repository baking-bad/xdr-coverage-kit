//! The scenarios: each one builds a source with the shapes in it.

mod contract_call;
mod contract_refusal;
mod contract_string;
mod empty_tx_set;
mod memo_text;
mod offer_update;

use crate::network::Network;
use Scenario::{Ledger, Synthetic};
use anyhow::Result;
use stellar_xdr::{LedgerCloseMeta, Type, TypeVariant};

/// The built Wasm of the contract `contracts/echo`.
const ECHO: &[u8] = include_bytes!("../../../contracts/wasm/echo.wasm");

/// The two ways to build a source.
pub enum Scenario {
    /// The scenario applies its transactions on a new network, and
    /// stellar-core closes the ledger that it returns.
    Ledger(fn(&mut Network) -> Result<Source<LedgerCloseMeta>>),
    /// The scenario builds a value that a network of one node cannot emit.
    /// It takes the network ID, which enters every signature.
    Synthetic(fn(network_id: [u8; 32]) -> Result<Source<Type>>),
}

/// Every scenario, by the name of its module.
pub const ALL: [(&str, Scenario); 6] = [
    ("contract_call", Ledger(contract_call::scenario)),
    ("contract_refusal", Ledger(contract_refusal::scenario)),
    ("contract_string", Ledger(contract_string::scenario)),
    ("memo_text", Ledger(memo_text::scenario)),
    ("offer_update", Ledger(offer_update::scenario)),
    ("empty_tx_set", Synthetic(empty_tx_set::scenario)),
];

/// A source as a scenario builds it: its value and its shapes, before
/// `generate` writes its files and its manifest entries.
pub struct Source<Value> {
    pub id: &'static str,
    pub shapes: Vec<Shape>,
    pub value: Value,
}

/// One rare shape of a source. The fields become a manifest entry.
pub struct Shape {
    pub id: &'static str,
    pub description: &'static str,
    /// The numbers of the CAPs that introduced the shape, ascending.
    pub caps: &'static [u16],
    pub since_protocol: u32,
    pub xdr_type: TypeVariant,
    pub json_pointer: String,
    pub sightings: &'static [&'static str],
}

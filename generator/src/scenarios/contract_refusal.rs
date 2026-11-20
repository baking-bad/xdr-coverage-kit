//! A contract call that fails, because the Soroban host refuses its
//! argument: a map with keys out of order.

use super::{ECHO, Shape, Source};
use crate::{
    contract::Contract,
    network::{Network, Outcome},
};
use anyhow::Result;
use stellar_xdr::{
    LedgerCloseMeta, ScMap, ScMapEntry, ScSymbol, ScVal, TypeVariant,
};

pub fn scenario(network: &mut Network) -> Result<Source<LedgerCloseMeta>> {
    let echo = Contract::deploy(network, ECHO)?;
    let call =
        network.apply(echo.call("echo", vec![map()?])?, Outcome::Failure)?;
    let argument = "/tx/tx/operations/0/body/invoke_host_function\
                    /host_function/invoke_contract/args/0";
    Ok(Source {
        id: "ledger-contract-refusal",
        shapes: vec![Shape {
            id: "scval-map-unsorted-keys",
            description: "A failed call keeps its arguments in the ledger, \
                    even arguments that the Soroban host refuses.",
            caps: &[46],
            since_protocol: 20,
            xdr_type: TypeVariant::ScVal,
            json_pointer: format!("{}{argument}", call.envelope),
            sightings: &[],
        }],
        value: call.ledger,
    })
}

/// A map with the keys `"a"` (symbol) and `1` (u32). The host orders
/// map keys by type first and puts a u32 before a symbol, so these keys
/// are out of order.
fn map() -> Result<ScVal> {
    let entry = |key, val| ScMapEntry {
        key,
        val: ScVal::U32(val),
    };
    let symbol = ScVal::Symbol(ScSymbol("a".try_into()?));
    let map =
        ScMap(vec![entry(symbol, 10), entry(ScVal::U32(1), 20)].try_into()?);
    Ok(ScVal::Map(Some(map)))
}

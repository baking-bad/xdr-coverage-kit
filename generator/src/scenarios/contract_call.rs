//! A contract call that returns a map with colliding keys.

use super::{ECHO, Shape, Source};
use crate::{
    contract::Contract,
    network::{Network, Outcome},
};
use anyhow::Result;
use stellar_xdr::{LedgerCloseMeta, ScMap, ScSymbol, ScVal, TypeVariant};

pub fn scenario(network: &mut Network) -> Result<Source<LedgerCloseMeta>> {
    let echo = Contract::deploy(network, ECHO)?;
    let call =
        network.apply(echo.call("echo", vec![map()?])?, Outcome::Success)?;
    let return_value = "/tx_apply_processing/v4/soroban_meta/return_value";
    Ok(Source {
        id: "ledger-contract-call",
        shapes: vec![Shape {
            id: "scval-map-colliding-keys",
            description: "A Soroban map can hold two keys that differ only \
                    in type and print the same, such as 1 and \"1\".",
            caps: &[46],
            since_protocol: 20,
            xdr_type: TypeVariant::ScVal,
            json_pointer: format!("{}{return_value}", call.processing),
            sightings: &[
                "https://github.com/stellar/js-stellar-sdk/issues/1789",
            ],
        }],
        value: call.ledger,
    })
}

/// A map with the keys `1` (u32) and `"1"` (symbol), which print the same
/// and are different values.
fn map() -> Result<ScVal> {
    let map = ScMap::sorted_from([
        (ScVal::U32(1), ScVal::U32(10)),
        (ScVal::Symbol(ScSymbol("1".try_into()?)), ScVal::U32(20)),
    ])?;
    Ok(ScVal::Map(Some(map)))
}

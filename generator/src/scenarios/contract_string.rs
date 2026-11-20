//! A contract call that returns a string whose bytes are not UTF-8.

use super::{ECHO, Shape, Source};
use crate::{
    contract::Contract,
    network::{Network, Outcome},
};
use anyhow::Result;
use stellar_xdr::{
    LedgerCloseMeta, ScMap, ScString, ScSymbol, ScVal, TypeVariant,
};

pub fn scenario(network: &mut Network) -> Result<Source<LedgerCloseMeta>> {
    let echo = Contract::deploy(network, ECHO)?;
    let call =
        network.apply(echo.call("echo", vec![map()?])?, Outcome::Success)?;
    let string = "/tx_apply_processing/v4/soroban_meta/return_value/map/0/val";
    Ok(Source {
        id: "ledger-contract-string",
        shapes: vec![Shape {
            id: "scval-string-non-utf8",
            description: "A Soroban string holds any bytes, UTF-8 or not.",
            caps: &[46],
            since_protocol: 20,
            xdr_type: TypeVariant::ScVal,
            json_pointer: format!("{}{string}", call.processing),
            sightings: &[
                "https://github.com/stellar/js-stellar-sdk/issues/1823",
            ],
        }],
        value: call.ledger,
    })
}

/// A map with one value: a string whose single byte `ff` is not valid
/// UTF-8. The Soroban host does not validate the bytes of a string.
fn map() -> Result<ScVal> {
    let text = ScVal::String(ScString(vec![0xff].try_into()?));
    let map = ScMap::sorted_from([(
        ScVal::Symbol(ScSymbol("text".try_into()?)),
        text,
    )])?;
    Ok(ScVal::Map(Some(map)))
}

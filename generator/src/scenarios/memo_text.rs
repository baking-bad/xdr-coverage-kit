//! A classic transaction with no preconditions, whose text memo is not UTF-8.

use super::{Shape, Source};
use crate::network::{Body, Network, Outcome};
use anyhow::Result;
use stellar_xdr::{
    BumpSequenceOp, LedgerCloseMeta, Memo, OperationBody, SequenceNumber,
    TransactionExt, TypeVariant,
};

pub fn scenario(network: &mut Network) -> Result<Source<LedgerCloseMeta>> {
    let applied = network.apply(body()?, Outcome::Success)?;
    Ok(Source {
        id: "ledger-memo-text",
        shapes: vec![
            Shape {
                id: "memo-text-non-utf8",
                description: "A text memo holds any bytes, UTF-8 or not.",
                caps: &[],
                since_protocol: 0,
                xdr_type: TypeVariant::Memo,
                json_pointer: format!("{}/tx/tx/memo", applied.envelope),
                sightings: &[
                    "https://github.com/stellar/js-stellar-sdk/issues/1823",
                ],
            },
            Shape {
                id: "transaction-no-preconditions",
                description: "A transaction can state no precondition, \
                    not even time bounds.",
                caps: &[15],
                since_protocol: 13,
                xdr_type: TypeVariant::Transaction,
                json_pointer: format!("{}/tx/tx", applied.envelope),
                sightings: &[
                    "https://github.com/stellar/js-stellar-sdk/issues/1911",
                ],
            },
        ],
        value: applied.ledger,
    })
}

/// The body of a classic transaction with one operation that changes nothing,
/// and a text memo whose single byte `ff` is not valid UTF-8. stellar-core
/// does not validate the bytes of a text memo.
fn body() -> Result<Body> {
    Ok(Body {
        fee: 100,
        memo: Memo::Text(vec![0xff].try_into()?),
        operation: OperationBody::BumpSequence(BumpSequenceOp {
            bump_to: SequenceNumber(0),
        }),
        ext: TransactionExt::V0,
    })
}

//! A transaction that updates an offer whose ID a 64-bit float cannot hold.

use super::{Shape, Source};
use crate::network::{Body, Network, Outcome};
use anyhow::Result;
use stellar_xdr::{
    AlphaNum4, Asset, AssetCode4, LedgerCloseMeta, ManageSellOfferOp, Memo,
    OperationBody, Price, TransactionExt, TypeVariant,
};

/// The first integer that a 64-bit float cannot hold: 2^53 + 1.
const OFFER_ID: i64 = 9_007_199_254_740_993;

pub fn scenario(network: &mut Network) -> Result<Source<LedgerCloseMeta>> {
    let applied = network.apply(body(network), Outcome::Failure)?;
    Ok(Source {
        id: "ledger-offer-update",
        shapes: vec![Shape {
            id: "operation-offer-id-above-2-53",
            description: "An offer ID is a 64-bit integer, and a 64-bit \
                    float cannot hold every integer above 2^53.",
            caps: &[],
            since_protocol: 0,
            xdr_type: TypeVariant::Operation,
            json_pointer: format!("{}/tx/tx/operations/0", applied.envelope),
            sightings: &[
                "https://github.com/stellar/js-stellar-sdk/issues/1785",
            ],
        }],
        value: applied.ledger,
    })
}

/// The body of a transaction that updates the offer `OFFER_ID`. stellar-core
/// numbers the offers of a network from 1, so no such offer exists. The
/// transaction enters the ledger with the ID, and applying it fails.
fn body(network: &Network) -> Body {
    Body {
        fee: 100,
        memo: Memo::None,
        operation: OperationBody::ManageSellOffer(ManageSellOfferOp {
            selling: Asset::Native,
            buying: Asset::CreditAlphanum4(AlphaNum4 {
                asset_code: AssetCode4(*b"USD\0"),
                issuer: network.root(),
            }),
            amount: 10_000_000,
            price: Price { n: 1, d: 1 },
            offer_id: OFFER_ID,
        }),
        ext: TransactionExt::V0,
    }
}

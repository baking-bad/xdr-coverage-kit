//! The value of a ledger whose transaction set the validators voted to drop.

use super::{Shape, Source};
use crate::codec::LIMITS;
use anyhow::Result;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use stellar_xdr::{
    EnvelopeType, Hash, LedgerCloseValueSignature, NodeId, PublicKey,
    Signature, StellarValue, StellarValueExt, StellarValueProposedMsValue,
    TimePoint, TimePointMs, Type, TypeVariant, Uint256, VecM, WriteXdr,
};

/// The close time of the value, in milliseconds since 1970.
const CLOSE_TIME_MS: u64 = 1_000_000_000_250;

pub fn scenario(network_id: [u8; 32]) -> Result<Source<Type>> {
    Ok(Source {
        id: "value-empty-tx-set",
        shapes: vec![Shape {
            id: "stellar-value-empty-tx-set",
            description: "A ledger can close without its transaction set: \
                    its value holds a zero tx_set_hash, and the extension \
                    holds the hash of the dropped set.",
            caps: &[83, 88],
            since_protocol: 30,
            xdr_type: TypeVariant::StellarValue,
            json_pointer: String::new(),
            sightings: &[
                "https://github.com/stellar/rs-stellar-archivist/pull/30",
            ],
        }],
        value: Type::StellarValue(Box::new(value(network_id)?)),
    })
}

/// The ledger closes empty: `tx_set_hash` is zero, and the extension holds
/// the hash of the dropped set. stellar-core builds this value only when
/// several validators wait too long for a set. A network of one node never
/// emits it, so the generator builds it.
fn value(network_id: [u8; 32]) -> Result<StellarValue> {
    let validator = SigningKey::from_bytes(&[3; 32]);
    let dropped = Hash(Sha256::digest("the dropped transaction set").into());
    let close_time = CLOSE_TIME_MS / 1000;

    // NOTE: the signature covers the value as a validator proposed it, with
    // the hash of the dropped set, as `HerderImpl::makeStellarValue` in
    // stellar-core signs it.
    let mut signed = network_id.to_vec();
    signed.extend(EnvelopeType::Scpvalue.to_xdr(LIMITS)?);
    signed.extend(dropped.to_xdr(LIMITS)?);
    signed.extend(close_time.to_xdr(LIMITS)?);
    signed.extend(CLOSE_TIME_MS.to_xdr(LIMITS)?);
    let signature = LedgerCloseValueSignature {
        node_id: NodeId(PublicKey::PublicKeyTypeEd25519(Uint256(
            validator.verifying_key().to_bytes(),
        ))),
        signature: Signature(validator.sign(&signed).to_vec().try_into()?),
    };
    Ok(StellarValue {
        tx_set_hash: Hash([0; 32]),
        close_time: TimePoint(close_time),
        upgrades: VecM::default(),
        ext: StellarValueExt::EmptyTxSetMs(StellarValueProposedMsValue {
            close_time_ms: TimePointMs(CLOSE_TIME_MS),
            tx_set_hash: dropped,
            previous_ledger_hash: Hash(Sha256::digest("ledger 1").into()),
            previous_ledger_version: 30,
            lc_value_signature: signature,
        }),
    })
}

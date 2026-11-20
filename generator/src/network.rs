//! A new network that a scenario drives: it applies the transactions of
//! the root account, each in a ledger of its own.

use crate::{
    codec,
    node::{Node, NodeConfig},
};
use anyhow::{Result, bail, ensure};
use ed25519_dalek::{Signer, SigningKey};
use stellar_xdr::{
    AccountId, DecoratedSignature, LedgerCloseMeta, Memo, MuxedAccount,
    Operation, OperationBody, Preconditions, PublicKey, SequenceNumber,
    Signature, SignatureHint, Transaction, TransactionEnvelope, TransactionExt,
    TransactionResultResult, TransactionV1Envelope, Uint256, VecM,
};

/// A running network of one node, upgraded to the protocol of its settings.
pub struct Network {
    node: Node,
    id: [u8; 32],
    protocol: u32,
    root: SigningKey,
    /// The sequence number of the next transaction of the root account.
    seq_num: i64,
}

/// The parts of a transaction that differ between the scenarios.
pub struct Body {
    pub fee: u32,
    pub memo: Memo,
    pub operation: OperationBody,
    pub ext: TransactionExt,
}

/// What a scenario expects of a transaction that enters a ledger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Applying the transaction succeeds.
    Success,
    /// The transaction enters the ledger, and applying it fails.
    Failure,
}

/// A transaction that the network applied, and its ledger.
pub struct Applied {
    pub ledger: LedgerCloseMeta,
    /// The JSON Pointer of the transaction envelope in the decoded ledger:
    /// what the account sent.
    pub envelope: String,
    /// The JSON Pointer of the processing record of the transaction in
    /// the decoded ledger: its result, its events and its ledger changes.
    pub processing: String,
}

impl Network {
    /// Starts a node on a new network, upgraded to the protocol of `config`.
    pub fn start(config: &NodeConfig) -> Result<Network> {
        let id = config.network_id();
        Ok(Network {
            node: Node::start(config)?,
            id,
            protocol: config.protocol,
            // NOTE: the network ID is also the seed of the root account.
            root: SigningKey::from_bytes(&id),
            seq_num: 1,
        })
    }

    /// The network ID: the SHA-256 hash of the passphrase.
    pub fn id(&self) -> [u8; 32] {
        self.id
    }

    /// The root account, which holds every lumen of a new network.
    pub fn root(&self) -> AccountId {
        let key = self.root.verifying_key().to_bytes();
        AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(key)))
    }

    /// Applies `body` as a transaction of the root account, alone in
    /// a new ledger. Fails unless the transaction ends as `expected`.
    pub fn apply(&mut self, body: Body, expected: Outcome) -> Result<Applied> {
        let envelope = self.sign(body)?;
        self.node.submit(&envelope)?;
        self.node.close()?;
        self.seq_num += 1;

        let ledger = self.node.last_ledger()?;
        let LedgerCloseMeta::V2(meta) = &ledger else {
            bail!("the ledger is not a LedgerCloseMeta v2");
        };
        let version = meta.ledger_header.header.ledger_version;
        ensure!(
            version == self.protocol,
            "the ledger closed under protocol {version}, and the network runs \
             protocol {}",
            self.protocol
        );
        let [processing] = meta.tx_processing.as_slice() else {
            bail!(
                "the ledger holds {} transactions, and a scenario applies one \
                 per ledger",
                meta.tx_processing.len()
            );
        };
        let outcome = match processing.result.result.result {
            TransactionResultResult::TxSuccess(_) => Outcome::Success,
            _ => Outcome::Failure,
        };
        ensure!(
            outcome == expected,
            "the transaction ended as {outcome:?}, and the scenario expects \
             {expected:?}: {}",
            serde_json::to_string(&processing.result.result)?
        );

        // NOTE: only the transaction set of a ledger holds envelopes, and this
        // ledger holds one transaction, so the envelope stands at one place.
        let mut places = Vec::new();
        codec::pointers_to(
            &serde_json::to_value(&ledger)?,
            &serde_json::to_value(&envelope)?,
            String::new(),
            &mut places,
        );
        let [envelope] = places.as_slice() else {
            bail!(
                "the ledger holds the envelope at {} places, and the scenario \
                 expects one: {places:?}",
                places.len()
            );
        };
        Ok(Applied {
            ledger,
            envelope: envelope.clone(),
            processing: "/v2/tx_processing/0".to_string(),
        })
    }

    /// A signed transaction of the root account with one operation.
    fn sign(&self, body: Body) -> Result<TransactionEnvelope> {
        let public = self.root.verifying_key().to_bytes();
        let operation = Operation {
            source_account: None,
            body: body.operation,
        };
        let mut envelope = TransactionV1Envelope {
            tx: Transaction {
                source_account: MuxedAccount::Ed25519(Uint256(public)),
                fee: body.fee,
                seq_num: SequenceNumber(self.seq_num),
                // NOTE: the entry `transaction-no-preconditions` takes its
                // shape from this line: a transaction with no precondition,
                // not even time bounds.
                cond: Preconditions::None,
                memo: body.memo,
                operations: vec![operation].try_into()?,
                ext: body.ext,
            },
            signatures: VecM::default(),
        };
        let hash = envelope.hash(self.id)?;
        let signature = DecoratedSignature {
            hint: SignatureHint(public[28..].try_into().expect("four bytes")),
            signature: Signature(self.root.sign(&hash).to_vec().try_into()?),
        };
        envelope.signatures = vec![signature].try_into()?;
        Ok(TransactionEnvelope::Tx(envelope))
    }
}

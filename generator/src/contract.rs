//! A Soroban contract on the network: its deployment and its calls.

use crate::{
    codec::LIMITS,
    network::{Body, Network, Outcome},
};
use anyhow::Result;
use sha2::{Digest, Sha256};
use stellar_xdr::{
    ContractDataDurability, ContractExecutable, ContractId, ContractIdPreimage,
    ContractIdPreimageFromAddress, CreateContractArgsV2, Hash, HashIdPreimage,
    HashIdPreimageContractId, HostFunction, InvokeContractArgs,
    InvokeHostFunctionOp, LedgerFootprint, LedgerKey, LedgerKeyContractCode,
    LedgerKeyContractData, Memo, OperationBody, ScAddress, ScSymbol, ScVal,
    SorobanAuthorizationEntry, SorobanAuthorizedFunction,
    SorobanAuthorizedInvocation, SorobanCredentials, SorobanResources,
    SorobanTransactionData, SorobanTransactionDataExt, TransactionExt, Uint256,
    VecM, WriteXdr,
};

// NOTE: no simulation runs here, so every Soroban transaction declares the
// most instructions and write bytes that a new network allows. stellar-core
// charges the fee of the declared resources and of the rent and events used,
// and refunds the rest of `RESOURCE_FEE`.
/// The CPU instructions that a Soroban transaction declares.
const INSTRUCTIONS: u32 = 2_500_000;
/// The bytes that it declares to write.
const WRITE_BYTES: u32 = 3_200;
/// The resource fee that it offers, in stroops.
const RESOURCE_FEE: i64 = 5_000_000;

/// The inclusion fee: the base fee of a network whose ledgers are not full.
const INCLUSION_FEE: u32 = 100;

/// A contract that the root account deployed.
pub struct Contract {
    code: Hash,
    address: ScAddress,
}

impl Contract {
    /// Uploads `wasm` and creates a contract from it, in two ledgers. The root
    /// account deploys it with the salt zero and no constructor arguments.
    pub fn deploy(network: &mut Network, wasm: &[u8]) -> Result<Contract> {
        let code = Hash(Sha256::digest(wasm).into());
        let create = CreateContractArgsV2 {
            contract_id_preimage: ContractIdPreimage::Address(
                ContractIdPreimageFromAddress {
                    address: ScAddress::Account(network.root()),
                    salt: Uint256([0; 32]),
                },
            ),
            executable: ContractExecutable::Wasm(code.clone()),
            constructor_args: VecM::default(),
        };
        let preimage = HashIdPreimage::ContractId(HashIdPreimageContractId {
            network_id: Hash(network.id()),
            contract_id_preimage: create.contract_id_preimage.clone(),
        });
        let id = Sha256::digest(preimage.to_xdr(LIMITS)?).into();
        let contract = Contract {
            code,
            address: ScAddress::Contract(ContractId(Hash(id))),
        };

        network.apply(
            transaction(
                HostFunction::UploadContractWasm(wasm.try_into()?),
                VecM::default(),
                Vec::new(),
                vec![contract.code_key()],
            )?,
            Outcome::Success,
        )?;
        // NOTE: the root account is the source of the transaction, so it
        // authorizes the creation as the source account.
        let auth = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: SorobanAuthorizedInvocation {
                function: SorobanAuthorizedFunction::CreateContractV2HostFn(
                    create.clone(),
                ),
                sub_invocations: VecM::default(),
            },
        };
        network.apply(
            transaction(
                HostFunction::CreateContractV2(create),
                vec![auth].try_into()?,
                vec![contract.code_key()],
                vec![contract.instance_key()],
            )?,
            Outcome::Success,
        )?;
        Ok(contract)
    }

    /// The body of a transaction that calls `function` with `args`. The
    /// footprint holds the code and the instance of the contract, so the
    /// function must not touch other storage.
    pub fn call(&self, function: &str, args: Vec<ScVal>) -> Result<Body> {
        let call = InvokeContractArgs {
            contract_address: self.address.clone(),
            function_name: ScSymbol(function.try_into()?),
            args: args.try_into()?,
        };
        transaction(
            HostFunction::InvokeContract(call),
            VecM::default(),
            vec![self.code_key(), self.instance_key()],
            Vec::new(),
        )
    }

    fn code_key(&self) -> LedgerKey {
        LedgerKey::ContractCode(LedgerKeyContractCode {
            hash: self.code.clone(),
        })
    }

    fn instance_key(&self) -> LedgerKey {
        LedgerKey::ContractData(LedgerKeyContractData {
            contract: self.address.clone(),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Persistent,
        })
    }
}

/// The body of a Soroban transaction: one host function and its footprint.
fn transaction(
    host_function: HostFunction,
    auth: VecM<SorobanAuthorizationEntry>,
    read_only: Vec<LedgerKey>,
    read_write: Vec<LedgerKey>,
) -> Result<Body> {
    let resources = SorobanResources {
        footprint: LedgerFootprint {
            read_only: read_only.try_into()?,
            read_write: read_write.try_into()?,
        },
        instructions: INSTRUCTIONS,
        // NOTE: since protocol 23 the contract code and data live in
        // memory, and reading them reads no disk.
        disk_read_bytes: 0,
        write_bytes: WRITE_BYTES,
    };
    Ok(Body {
        fee: u32::try_from(RESOURCE_FEE)? + INCLUSION_FEE,
        memo: Memo::None,
        operation: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function,
            auth,
        }),
        ext: TransactionExt::V1(SorobanTransactionData {
            ext: SorobanTransactionDataExt::V0,
            resources,
            resource_fee: RESOURCE_FEE,
        }),
    })
}

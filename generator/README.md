# Generator

The generator produces the corpus and verifies it, with two commands:

- [`cargo run -- generate`](#generate) produces the corpus on a local stellar-core.
- [`cargo run -- check`](#check) verifies the corpus against its manifest.

Both commands run from the root of the repository. The option `--corpus` names another directory of the corpus.

## `generate`

`generate` runs every scenario in `src/scenarios/` and writes the source files, the manifest and its JSON Schema. Then it runs `check`.

A scenario builds one source. It receives a new local network, applies its transactions and returns one ledger with the shapes in it. Each transaction stands alone in a ledger of its own, and the scenario names the outcome that it expects of each. Any other outcome stops the generation.

<details>
<summary>A scenario: a contract call that returns a map with colliding keys</summary>

```rust
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
```

</details>

`generate` needs stellar-core 30.0.0: the program `stellar-core` on `PATH`, or the path in the option `--stellar-core` or in the variable `STELLAR_CORE`. It stops on any other release. It needs no access to a network.

For each scenario the generator starts a new network of one node and closes every ledger at a fixed close time. So a source depends neither on the other scenarios nor on the time of the run.

## `check`

`check` needs no stellar-core. It applies these rules:

- **schema**: the schema file equals the schema that the Rust types of the manifest give.
- **source**, **entry**: the `id` holds lower-case letters, digits and hyphens alone, and no other source or entry has the same `id`.
- **source**: the bytes decode to the JSON file, and the JSON encodes to the same bytes.
- **source**: the header of a ledger carries the protocol that `ledger_protocol` states.
- **entry**: `location.source` names a source of the manifest, and the byte range lies inside its bytes.
- **entry**: the byte range decodes to the value at the JSON Pointer.
- **entry**: `since_protocol` is not above the protocol of the ledger.
- **entry**: `soroban_validity` is the answer of the Soroban host (`soroban-env-host`).

A file of `sources/` that no source of the manifest has gives a warning, and `check` still passes.

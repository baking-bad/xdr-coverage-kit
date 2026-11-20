# xdr-coverage-kit

[![CI](https://github.com/baking-bad/xdr-coverage-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/baking-bad/xdr-coverage-kit/actions/workflows/ci.yml)

A test kit for applications that read the Stellar ledger. It holds a corpus of rare but valid XDR and a generator that produces the corpus on a local stellar-core. A runner replays the corpus through an SDK and reports where the SDK changes a value.

This repository is a prototype for the SCF RFP "XDR/CAP-Coverage Audit Tool and Indexer-Hardening Guide".

## The design

**A neutral corpus of rare data.** The corpus is tied to no language and no SDK. A source is a whole value that the network emits, in most cases a ledger, as bytes and as its decoded value. An entry names one rare shape by its place in a source: a path into the decoded value and a byte range.

**Checks follow the structure of the data.** Every entry has an XDR type. A runner holds its checks by type, and one check exercises the group of SDK functions that work with that type. So the kit has three layers, and each layer is reused by the next:

| Layer    | Holds                                           | Example                                                                             |
| -------- | ----------------------------------------------- | ----------------------------------------------------------------------------------- |
| corpus   | rare data                                       | an operation that names the offer 2^53 + 1                                          |
| XDR type | the structure of that data                      | `Operation`                                                                         |
| check    | the SDK functions that work with that structure | the SDK reads the operation and builds it again, with any of its operation builders |

A check compares what the SDK returns with what the network wrote. A new entry of a known type goes through the checks that exist.

**The data comes from stellar-core.** The entries are cut from ledgers that a local stellar-core closed. The corpus takes a synthetic value only when a real network can emit it and one local node cannot.

**A new shape is one small scenario.** A scenario is a short Rust function: it applies transactions on a local network and names the shape in the ledger that it gets. All scenarios share the code that runs the node and signs the transactions.

## What it shows

The RFP asks the runner to reproduce three defects of the JavaScript SDK on the release before their fix. The kit reproduces two of them, on the last release before the fix and on the release that ships it.

| Defect                                                         | Entry                           | Check              | 17.2.1 | 17.3.0  |
| -------------------------------------------------------------- | ------------------------------- | ------------------ | ------ | ------- |
| [#1789](https://github.com/stellar/js-stellar-sdk/issues/1789) | `scval-map-colliding-keys`      | `scValToNative`    | lost   | refused |
| [#1785](https://github.com/stellar/js-stellar-sdk/issues/1785) | `operation-offer-id-above-2-53` | `rebuild(numbers)` | lost   | refused |

In #1789 `scValToNative` drops a map entry when two keys print the same, and in #1785 an offer ID above 2^53, passed as a number, is encoded as another ID. `lost` means that the SDK returned another value with no error, and `refused` means that it raised an error. Each fix makes the SDK raise an error where it lost the value before. Release 17.2.1 is older than protocol 30 and cannot decode a whole ledger of the corpus. An entry gives the bytes of its shape alone, so the release is still tested on the shape. CI runs this comparison on every push and fails when the verdict of either release changes.

The same checks give two verdicts that no issue of the SDK reports. Both verdicts are the same on releases 17.2.1, 17.3.0 and 17.4.0.

| Shape                                       | Entry                          | Check           | Verdict |
| ------------------------------------------- | ------------------------------ | --------------- | ------- |
| A Soroban string whose bytes are not UTF-8. | `scval-string-non-utf8`        | `contract.Spec` | lost    |
| A transaction with no preconditions.        | `transaction-no-preconditions` | `cloneFrom`     | refused |

For the string, `contract.Spec` returns text with U+FFFD in place of a byte, and `scValToNative` keeps the bytes. For the transaction, the SDK decodes it, and `TransactionBuilder.cloneFrom` does not build it again, because the builder requires time bounds. Each shape has a related issue. [#1823](https://github.com/stellar/js-stellar-sdk/issues/1823) describes the same read of a string in the generated `.value` getters, and [#1911](https://github.com/stellar/js-stellar-sdk/issues/1911) reports the same requirement of time bounds in another function. The corpus states that the network carries the shape, and the team of the SDK decides whether the verdict is a defect.

## Parts

| Directory                         | Holds                                                                         |
| --------------------------------- | ----------------------------------------------------------------------------- |
| [`corpus/`](corpus)               | The manifest, its JSON Schema, and the source files in `sources/`.            |
| [`generator/`](generator)         | The generator of the corpus: `generate` produces it, and `check` verifies it. |
| [`runners/js/`](runners/js)       | The runner for the JavaScript SDK.                                            |
| [`reproductions/`](reproductions) | The script that reproduces the two defects on the releases around their fix.  |
| [`contracts/`](contracts)         | The Soroban contracts that the scenarios deploy.                              |

## Run it

From the root of the repository:

```sh
cargo run -- check          # verify the corpus
cargo run -- generate       # produce the corpus again, on stellar-core 30.0.0

(cd runners/js && npm install && npm start -- --known known.json)
(cd reproductions/js-stellar-sdk-17.3.0 && npm install && npm start)
```

The reproduction uses the runner, so it runs after the line of the runner. The first `cargo run` builds the generator, which takes about a minute.

| To run                          | Install                                                                                                                                  |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `check`                         | Rust through [rustup](https://rustup.rs), which reads the release from `rust-toolchain.toml`.                                            |
| `generate`                      | stellar-core 30.0.0 as the program `stellar-core`. CI takes it from the Docker image `stellar/stellar-core:30.0.0-3642.0e49751aa.noble`. |
| the runner and the reproduction | Node.js 22.18 or later.                                                                                                                  |
| a build of a contract           | The [Stellar CLI](https://developers.stellar.org/docs/tools/cli) 30.                                                                     |

## Continuous integration

A push or a pull request runs these jobs. The whole run needs no access to a Stellar network.

| Job             | What it proves                                                                                                                                  |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `generator`     | The generator passes its lint and its tests, and `check` passes on the corpus.                                                                  |
| `runner`        | The runner passes its tests on Node.js 22 and 26, and its run on the installed SDK gives the verdicts of [`known.json`](runners/js/known.json). |
| `reproductions` | The releases before and after the fix of #1789 and #1785 give the verdicts that the reproduction states.                                        |
| `corpus`        | The corpus comes out of the generator: a new run of the scenarios on stellar-core changes no file.                                              |
| `contracts`     | The Wasm of each contract comes out of its source. The job runs when a contract changes.                                                        |

The jobs stand in [`.github/workflows/`](.github/workflows).

## Not in this prototype

The prototype covers the corpus, the generator and one runner. The kit that the RFP describes also needs:

- Runners for the Go and Rust SDKs, and the matrix that sets the verdicts of the SDKs side by side.
- The replay harness for indexers, the linter and the guide.
- The third defect that the RFP names for the runner, js-stellar-sdk#1766.
- Sources from network history, and generation under the testnet and mainnet passphrases.
- A fixed order for the lists of ledger entry changes, which stellar-core writes in a varying order ([stellar-core#5507](https://github.com/stellar/stellar-core/issues/5507)).
- The comparison with the golden ledgers of stellar-core's own tests (`--check-lcm`).

## License

Apache-2.0. Copyright 2026 BB Strategy Pte. Ltd., which builds the kit as Baking Bad. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).

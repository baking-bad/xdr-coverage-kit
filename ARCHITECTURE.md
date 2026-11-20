# Architecture

XDR Coverage Kit is a test kit for software that reads the Stellar ledger: SDKs, indexers and the pipelines of SDF. It answers the RFP "XDR/CAP-Coverage Audit Tool and Indexer-Hardening Guide" of the Stellar Community Fund. It has six parts, and the Status table at the end states what is built and what each tranche adds.

1. Corpus and generator
2. SDK conformance runner
3. Replay harness and reference adapter
4. Indexer-hardening guide
5. Findings reports
6. Maintenance through protocol upgrades

## The system

In the diagrams a rectangle is a part of the kit, and a rounded box is software outside the kit. A dashed box or a dashed arrow is planned, and the box names the tranche that brings it.

```mermaid
%%{init: {"flowchart": {"wrappingWidth": 400}}}%%
flowchart TB
  core(["stellar-core"])
  generator["generator"]
  corpus["corpus"]
  guide["indexer-hardening guide<br/><i>tranches 2 and 3</i>"]:::planned
  subgraph runners["SDK conformance runner"]
    js["JS runner"]
    gors["Go and Rust runners<br/><i>tranche 1</i>"]:::planned
  end
  harness["replay harness and adapter<br/><i>tranches 2 and 3</i>"]:::planned
  sdks(["SDKs"])
  indexers(["indexers"])
  matrix["divergence matrix<br/><i>tranche 1</i>"]:::planned
  findings["findings reports<br/><i>tranches 2 and 3</i>"]:::planned

  core -->|ledgers| generator
  generator --> corpus
  generator -.->|field table| guide
  corpus --> runners
  corpus -.-> harness
  runners -->|calls| sdks
  runners -.->|verdicts| matrix
  harness -.->|feeds| indexers
  matrix -.-> findings
  harness -.->|results| findings



  style runners fill:none,stroke:#888
  classDef planned stroke-dasharray: 6 4
```

The generator builds the corpus from ledgers that stellar-core closes. Runners give the corpus to SDKs, and the harness gives it to indexers. Their failures become findings reports.

## Corpus and generator

### Data model

```mermaid
%%{init: {"flowchart": {"wrappingWidth": 400}}}%%
flowchart TB
  scenario["scenario<br/>a Rust function"]
  core(["stellar-core<br/>one local node"])
  lake(["Galexie<br/>public data lake"])

  subgraph manifest["manifest.json"]
    msrc["sources<br/>id, xdr_type, origin"]
    ment["entries<br/>id, xdr_type, caps, location"]
  end
  subgraph files["files of one source"]
    xdr["&lt;id&gt;.xdr<br/>bytes"]
    json["&lt;id&gt;.json<br/>decoded value"]
  end

  scenario -->|transactions| core
  scenario -->|names the shapes| manifest
  scenario -->|synthetic value| files
  core -->|closed ledger| files
  lake -.->|"ledger of a network<br/>tranche 1"| files
  msrc ~~~ ment
  msrc -->|id| files
  ment -->|"byte range, JSON Pointer"| files


  style files fill:none,stroke:#888
  style manifest fill:none,stroke:#888
  classDef planned stroke-dasharray: 6 4
```

The corpus is files: `corpus/manifest.json`, its JSON Schema, and the files of each source in `corpus/sources/`. The generator writes every file, and nobody edits them by hand.

A **source** is one whole XDR value that the network emits, in most cases a ledger. `<id>.xdr` holds its bytes, and `<id>.json` holds its decoded value in XDR-JSON (SEP-0051).

| Origin         | Meaning                                                                                                                             | Status    |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------- | --------- |
| `stellar_core` | A ledger that a local stellar-core closed, with the version of stellar-core and the protocol of the ledger.                         | built     |
| `synthetic`    | A value that the generator built, because a real network can emit it and one local node cannot.                                     | built     |
| `history`      | A ledger from the history of testnet or mainnet, with its network and its sequence number, taken from the Galexie public data lake. | tranche 1 |

An **entry** is one rare shape: one form of an XDR value that a reader seldom meets. The manifest records for it:

| Field              | Meaning                                                                                                                         |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| `id`               | The name of the entry.                                                                                                          |
| `description`      | The fact that a reader of the shape must keep.                                                                                  |
| `caps`             | The CAPs that introduced the shape. The list is empty for a shape that the network had at launch.                               |
| `since_protocol`   | The first protocol under which the network can emit the shape, 0 for a shape that the network had at launch.                    |
| `xdr_type`         | The XDR type of the shape. It selects the checks.                                                                               |
| `soroban_validity` | Whether `soroban-env-host`, the Soroban host that stellar-core runs, accepts the value: `valid`, `invalid` or `not_applicable`. |
| `location`         | The source, a JSON Pointer into its decoded value, and a byte range in its bytes.                                               |
| `sightings`        | The upstream issues and pull requests where the shape broke a reader.                                                           |

Tranche 1 replaces a synthetic value with a `history` ledger wherever the history holds the shape.

An entry holds no bytes of its own. So each ledger is stored once, and every shape stays in the context where the network wrote it. A reader that cannot decode the whole ledger still reads the shape with one slice. For example, the JavaScript SDK 17.2.1 predates protocol 30 and still reads each shape.

The entries of the corpus:

| Entry                           | XDR type       | Source                    |
| ------------------------------- | -------------- | ------------------------- |
| `scval-map-colliding-keys`      | `ScVal`        | `ledger-contract-call`    |
| `scval-map-unsorted-keys`       | `ScVal`        | `ledger-contract-refusal` |
| `scval-string-non-utf8`         | `ScVal`        | `ledger-contract-string`  |
| `memo-text-non-utf8`            | `Memo`         | `ledger-memo-text`        |
| `transaction-no-preconditions`  | `Transaction`  | `ledger-memo-text`        |
| `operation-offer-id-above-2-53` | `Operation`    | `ledger-offer-update`     |
| `stellar-value-empty-tx-set`    | `StellarValue` | `value-empty-tx-set`      |

stellar-core keeps golden ledgers of its own unit tests, which the RFP names by their option `--check-lcm`. Tranche 1 records, for each entry, the golden ledgers that hold the same shape, and runs the checks of the corpus on them.

### Generator

A **scenario** is a short Rust function. It receives a new local network of one stellar-core node, applies its transactions and returns one ledger with its shapes named. The scenario names the outcome it expects of each transaction, and any other outcome stops the generation.

The node closes every ledger at a fixed close time, so a source depends neither on other scenarios nor on the time of the run.

`generate` runs every scenario and writes the corpus. It stops on any release of stellar-core other than 30.0.0, because another release may close other bytes. `check` verifies the corpus with no stellar-core:

- The schema file equals the schema that the Rust types of the manifest give.
- Every `id` has one form, and no two items share an `id`.
- The bytes and the JSON of each source encode to each other, and the header of a ledger carries the protocol that the manifest states.
- The byte range of each entry lies in its source and decodes to the value at its JSON Pointer.
- `since_protocol` is not above the protocol of the ledger.
- `soroban_validity` is the answer of `soroban-env-host`.

The network passphrase is a setting of the generator, and the manifest records it. Tranche 1 makes it an option of `generate`, so the same scenarios produce a testnet corpus and a mainnet corpus. A ledger from the history of a network carries the passphrase of that network, so it stands in a corpus of that network. The protocol version is a setting too, so a source can be closed under an earlier protocol for an older indexer, and each source records its protocol.

stellar-core writes some lists of ledger entry changes in a varying order (stellar-core#5507). Tranche 1 sorts them, so a source holds one fixed order.

Tranche 1 adds the coverage report, which lists for each CAP the entries that cover its shapes. It is built from the CAP list and the manifest.

## SDK conformance runner

A runner reads the corpus through one SDK. A **check** runs the SDK on one source or one entry and compares the result with the corpus. Two **levels** run on every XDR type, and a **helper** serves one type:

- `codec` reads the bytes into an SDK object and writes them back.
- `json` writes the object as XDR-JSON, compares it with the decoded value of the corpus, and reads that value back into bytes.
- A **helper** serves one XDR type and exercises the SDK functions that an application calls for that type. For example, `rebuild` reads any kind of operation and builds it again with the SDK's own builder.

```mermaid
%%{init: {"flowchart": {"wrappingWidth": 400}}}%%
flowchart LR
  hc["<b>corpus</b>"]:::head ~~~ ht["<b>XDR type</b>"]:::head ~~~ hk["<b>check of the JS runner</b>"]:::head

  e1["scval-map-colliding-keys"]
  e2["scval-map-unsorted-keys"]
  e3["scval-string-non-utf8"]
  e4["memo-text-non-utf8"]
  e5["transaction-no-preconditions"]
  e6["operation-offer-id-above-2-53"]
  e7["stellar-value-empty-tx-set"]
  srcs["6 sources"]

  subgraph types[" "]
    t1["ScVal"]
    t2["Memo"]
    t3["Transaction"]
    t4["Operation"]
    t5["StellarValue"]
    tl["LedgerCloseMeta"]
  end

  h1["scValToNative"]
  h2["contract.Spec"]
  h3["union.value"]
  h4["Memo.value"]
  h5["cloneFrom"]
  h6["rebuild"]
  h7["rebuild(numbers)"]
  subgraph levels["levels: every XDR type"]
    codec["codec"]
    json["json"]
  end

  e1 --> t1
  e2 --> t1
  e3 --> t1
  e4 --> t2
  e5 --> t3
  e6 --> t4
  e7 --> t5
  srcs --> t5
  srcs --> tl
  t1 --> h1
  t1 --> h2
  t1 --> h3
  t2 --> h3
  t2 --> h4
  t3 --> h5
  t4 --> h6
  t4 --> h7
  types --> levels


  style types fill:none,stroke:#888
  style levels fill:none,stroke:#888
  classDef head fill:none,stroke:none
```

The kit has three layers, and each layer is reused by the next:

| Layer    | Holds                                           | Example                                                                             |
| -------- | ----------------------------------------------- | ----------------------------------------------------------------------------------- |
| corpus   | rare data                                       | an operation that names the offer 2^53 + 1                                          |
| XDR type | the structure of that data                      | `Operation`                                                                         |
| check    | the SDK functions that work with that structure | the SDK reads the operation and builds it again, with any of its operation builders |

A check cuts the bytes of the entry by its byte range and takes its decoded value by its JSON Pointer. It compares the result of the SDK with both and gives a **verdict**. The **known verdicts** of the SDK release then decide the run.

```mermaid
%%{init: {"flowchart": {"wrappingWidth": 400}}}%%
flowchart TB
  check["check"]
  sdk(["@stellar/stellar-sdk"])
  known(["known verdicts<br/>one file per SDK release<br/>kept by the SDK team"])
  listed{{"in known verdicts?"}}
  which{{"verdict?"}}
  same{{"same verdict as listed?"}}
  fail1["run fails"]
  pass["run passes"]
  fail2["run fails"]

  check -->|calls| sdk
  sdk -->|"result, compared<br/>with the corpus"| listed
  known --> listed
  listed -->|no| which
  listed -->|yes| same
  which -->|"lost: another value, no error"| fail1
  which -->|"refused: an error"| fail1
  which -->|"kept: the same value"| pass
  which -->|"skipped: no rule for this value"| pass
  same -->|yes| pass
  same -->|"no: the SDK changed"| fail2


```

Each known verdict states its reason.

The **divergence matrix** sets the verdicts of the JavaScript, Go and Rust runners side by side for each entry. A divergence is an entry that one SDK keeps and another loses or refuses.

A trial ran the seven entries through the JavaScript SDK, go-stellar-sdk and the Rust crates `stellar-xdr` and `soroban-spec-tools`. All three read every entry and every source and write the same bytes back. It found two failures that the JavaScript runner alone cannot show. The Rust crate `soroban-spec-tools` turns the map with colliding keys into a map of one key, the loss that js-stellar-sdk#1789 fixed in JavaScript. The builder of go-stellar-sdk refuses the transaction with no preconditions, as the JavaScript builder does. Neither is reported upstream.

## Replay harness and reference adapter

The harness feeds corpus ledgers to an unmodified indexer through an input that existing pipelines already read. It starts the indexer with the passphrase of the corpus, so one corpus serves the SDKs and the indexers. It replays each source alone, into a fresh database where the harness sets the cursor of the indexer to the ledger before the source. So a halt on one entry blocks no other entry.

- **Galexie data-lake layout**, first, in tranche 2. The harness writes each ledger as a Galexie file, with a `.config.json` that names the passphrase. A read-only S3-compatible endpoint serves the files, and wallet-backend reads them over S3.
- **Mock `getLedgers` endpoint**, second, in tranche 3. A JSON-RPC server answers `getHealth` and `getLedgers` from the same ledgers, for an indexer that reads only stellar-rpc. In a trial both inputs gave wallet-backend the same bytes of each ledger and the same stored rows.

An **adapter** connects one indexer to the harness. It builds the indexer at a commit and lets it ingest one source. It then reads the database and reports the stored items in one form for every indexer. Stored items include transaction hashes, participants, addresses, signer keys, trustlines and muxed IDs. A trial read the first three from wallet-backend. The reference adapter is the adapter for wallet-backend.

An indexer stores a part of a ledger. The harness says `mishandled` only about a shape that the indexer stores. For any other shape it reports whether the indexer advanced, halted, timed out or ran out of memory.

A shape that depends on an earlier ledger, such as a token balance, needs a source of several ledgers.

```mermaid
%%{init: {"flowchart": {"wrappingWidth": 400, "nodeSpacing": 30, "rankSpacing": 40}}}%%
flowchart TB
  subgraph corpus["corpus"]
    source["source<br/>a whole ledger"]
    expected["expected indexer result<br/>of each entry<br/><i>tranche 2</i>"]:::planned
  end

  subgraph inputs["replay harness: inputs"]
    lake["Galexie data-lake layout<br/>read-only S3 endpoint<br/><i>tranche 2</i>"]:::planned
    rpc["mock getLedgers endpoint<br/>JSON-RPC<br/><i>tranche 3</i>"]:::planned
  end

  indexer(["indexer, unmodified<br/>wallet-backend"])
  db(["wallet-backend database<br/>PostgreSQL"])
  adapter["adapter for wallet-backend<br/><i>tranche 2</i>"]:::planned
  compare["replay harness: comparison<br/><i>tranche 2</i>"]:::planned

  subgraph result["result of each entry"]
    pass["pass"]:::planned
    mishandled["mishandled<br/>a stored item with a wrong value"]:::planned
    dropped["dropped<br/>a halt or a missing item"]:::planned
  end

  source -.-> lake
  source -.-> rpc
  lake -.-> indexer
  rpc -.-> indexer
  indexer -.->|stores| db
  db -.->|stored items| adapter
  indexer -.->|halt| adapter
  adapter -.-> compare
  expected -.-> compare
  compare -.-> pass
  compare -.-> mishandled
  compare -.-> dropped


  style corpus fill:none,stroke:#888
  style inputs fill:none,stroke:#888
  style result fill:none,stroke:#888
  classDef planned stroke-dasharray: 6 4
```

A live indexer does not exit, so the adapter stops it when the cursor reaches the target ledger.

The releases of wallet-backend before the three fixes that the RFP names read no ledger of protocol 30. So the kit closes the sources for them under protocol 28, on the same stellar-core 30.0.0.

A fault of the harness or of the indexer setup gives no result and fails the run. A CI job replays the full corpus under a fixed memory limit.

## Where the expected values come from

| Check                   | Expected value                                        | Origin                                                                                                                                                                                                                                                     |
| ----------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A check of an SDK       | The bytes of the shape and their decoded value        | The bytes that stellar-core wrote. The crate `stellar-xdr` writes the decoded value, so the Rust `json` level checks that library against itself. The JavaScript SDK reads the decoded value independently.                                                |
| A result of the harness | The items that a correct indexer stores for the entry | The ledger itself. The generator derives an item from the ledger where a rule gives it, such as a transaction hash or a changed account in the meta. The scenario states an item that needs the meaning of the transaction, and the manifest records both. |

## Indexer-hardening guide

The guide is a document for indexer authors, written for developers.stellar.org. Its outline: validated and unvalidated fields, depth and size caps, participants from the meta, muxed and signed-payload addresses with the SEP-0023 forms, and a checklist. The checklist states the ingestion rules for indexers in any language. It replaces the linter that the RFP asks for, which this proposal leaves out.

The field table says which fields stellar-core validates and which it passes through. A row for a field that stellar-core passes through names the corpus entries where it accepted a rare value in that field. A row for a field that stellar-core validates cites the source of stellar-core. A test in CI checks the table against the manifest.

## Findings reports

A findings report holds one record per finding: the issue link, the corpus entry that reproduces it, severity, the CAP and the component. Every failure in the matrix and in the harness report maps to a filed issue. A defect in one SDK gets a sibling issue in each SDK that shares it. A finding that could halt or corrupt a production pipeline goes first through that project's security policy.

## Maintenance through protocol upgrades

| Part                    | Cost of a new CAP                                                                                                                               |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Corpus and generator    | One scenario for each new shape, on the new releases of stellar-core and stellar-xdr.                                                           |
| SDK conformance runner  | Nothing for a known type. A new type goes through `codec` and `json` at once. A helper needs code only when the type has its own SDK functions. |
| Replay harness          | A new kind of stored item only when the CAP adds a new kind of stored data.                                                                     |
| Indexer-hardening guide | New rows in the field table, checked by the same test.                                                                                          |

A scheduled CI job reads each new release of the XDR definitions of the protocol and compares it with the coverage report. It opens an issue in this repository for each new CAP or union arm that has no entry. A second scheduled run checks each new SDK release and the current wallet-backend against the corpus.

Baking Bad maintains the kit for 12 months after release 1.0 and through the first protocol upgrade after delivery. Release 1.0 adds a reusable CI job, so an SDK repository runs the corpus in its own CI with its own known verdicts.

## Design decisions

**A neutral corpus of rare data.** The corpus is tied to no language and no SDK. It holds values, and the runner of a language holds the forms in which that language passes them.

**Checks follow the structure of the data.** A runner holds its checks by XDR type, and one check exercises the group of SDK functions that work with that type. A new entry of a known type goes through the checks that exist.

**The data comes from stellar-core.** The entries are cut from ledgers that a local stellar-core closed. The corpus takes a synthetic value only when a real network can emit it and one local node cannot.

**A new shape is one small scenario.** A scenario is a short Rust function: it applies transactions on a local network and names the shape in the ledger that it gets.

**The network decides what is valid, and the team of an SDK decides what it accepts.** The corpus states that the network carries a value.

**A source is a whole value.** In most cases it is a ledger. The harness gives it to an indexer with its bytes unchanged, and every shape keeps the context where the network wrote it.

## Technical stack

| Part                  | Technology                                                                                                 |
| --------------------- | ---------------------------------------------------------------------------------------------------------- |
| Generator and `check` | Rust 1.100.0, edition 2024. The crates `stellar-xdr` 30.0.0 and `soroban-env-host` 30.0.1, pinned exactly. |
| Local network         | stellar-core 30.0.0. CI takes it from the image `stellar/stellar-core:30.0.0-3642.0e49751aa.noble`.        |
| Contracts             | Soroban contracts in Rust, built with the Stellar CLI 30.                                                  |
| JS runner             | TypeScript on Node.js 22.18 or later, `@stellar/stellar-sdk` 17.4.0. CI runs it on Node.js 22 and 26.      |
| Go and Rust runners   | go-stellar-sdk, and the Rust crates `stellar-xdr` and `soroban-spec-tools`, each at its newest release.    |
| Reference adapter     | wallet-backend, and PostgreSQL 17 with TimescaleDB 2.28, in containers.                                    |
| CI                    | GitHub Actions on Ubuntu 24.04.                                                                            |

## Infrastructure, users and license

**Infrastructure.** The kit is files and CI jobs in one repository. Every test runs offline, with no access to a Stellar network, no keys and no funds.

**User tracking.** The kit collects no telemetry and tracks no users.

**Decentralization.** The kit is a test tool with no running service, so it has nothing to decentralize. Anyone can run the generator on stellar-core and compare the corpus byte by byte.

**License.** Apache-2.0.

## Status

Status on 2026-11-20.

| Part                                  | Built                                                                                                                                                                                                                                                                                                                            | Planned                                                                                                                                                                                                                                        |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Corpus and generator                  | 7 entries in 6 sources: five ledgers closed by stellar-core 30.0.0, one synthetic `StellarValue`. `generate`, `check`, the JSON Schema of the manifest.                                                                                                                                                                          | Tranche 1: CAP list, coverage report, the shapes that the RFP names, both passphrases, network history, a fixed order of the lists of changes, the cross-reference to stellar-core's golden ledgers. Tranche 3: every CAP through protocol 30. |
| SDK conformance runner                | The JS runner with known verdicts: two levels and seven helpers for four XDR types. It reproduces js-stellar-sdk#1789 and #1785 on 17.2.1 and 17.3.0.                                                                                                                                                                            | Tranche 1: Go and Rust runners, divergence matrix, js-stellar-sdk#1766.                                                                                                                                                                        |
| Replay harness and reference adapter  | Nothing in the repository. A trial put each of the five ledgers of the corpus through an unmodified wallet-backend, from the Galexie layout over S3, in under half a second. A second trial reproduced wallet-backend#729, #731 and #716 on their pre-fix commits, on ledgers that stellar-core 30.0.0 closed under protocol 28. | Tranche 2: Galexie input, adapter interface, wallet-backend adapter. Tranche 3: mock `getLedgers` input.                                                                                                                                       |
| Indexer-hardening guide               | —                                                                                                                                                                                                                                                                                                                                | Tranche 2: draft. Tranche 3: reviewed guide.                                                                                                                                                                                                   |
| Findings reports                      | —                                                                                                                                                                                                                                                                                                                                | Tranche 2, updated in tranche 3.                                                                                                                                                                                                               |
| Maintenance through protocol upgrades | CI on every push regenerates the corpus and fails on any changed byte. It runs the JS runner against its known verdicts, and the reproductions.                                                                                                                                                                                  | Tranche 2: threat model and monitoring plan. Tranche 3: release 1.0, runbook, scheduled jobs.                                                                                                                                                  |

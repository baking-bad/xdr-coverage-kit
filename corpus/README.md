# Corpus

The corpus holds the manifest, its JSON Schema, and the source files in `sources/`.

> [!NOTE]
> The generator writes every file of the corpus. Nobody edits them by hand.

`manifest.json` holds two lists. A source is one XDR value, and its files are in `sources/`. An entry is one rare shape, and it names a place in a source.

```json
{
  "$schema": "manifest.schema.json",
  "network_passphrase": "Standalone Network ; February 2017",
  "sources": [],
  "entries": []
}
```

`network_passphrase` is the network of every ledger in the corpus. `manifest.schema.json` is the JSON Schema of the manifest.

## Sources

A source is one XDR value in two files: `sources/<id>.xdr` holds the bytes, and `sources/<id>.json` holds the decoded value in XDR-JSON ([SEP-0051](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0051.md)).

```json
{
  "id": "ledger-contract-call",
  "xdr_type": "LedgerCloseMeta",
  "origin": { "stellar_core": { "version": "30.0.0", "ledger_protocol": 30 } }
}
```

| Field      | Meaning                                          |
| ---------- | ------------------------------------------------ |
| `id`       | The name of the source and of its two files.     |
| `xdr_type` | The XDR type of the value.                       |
| `origin`   | Where the bytes come from, one of the two below. |

A source has one of two origins:

- `stellar_core`: a ledger that a local stellar-core closed. The origin records the version of stellar-core and `ledger_protocol`, the protocol that closed the ledger.
- `synthetic`: a value that the generator built, because a real network can emit it and one local node cannot.

## Entries

An entry holds no bytes of its own. It names its place in a source in two ways: a JSON Pointer into the decoded value, and a byte range. A reader gets the bytes of the shape with one slice. So a reader that cannot decode the whole ledger still reads the shape.

```json
{
  "id": "scval-map-colliding-keys",
  "description": "A Soroban map can hold two keys that differ only in type and print the same, such as 1 and \"1\".",
  "caps": ["CAP-0046"],
  "since_protocol": 20,
  "xdr_type": "ScVal",
  "soroban_validity": "valid",
  "location": {
    "source": "ledger-contract-call",
    "json_pointer": "/v2/tx_processing/0/tx_apply_processing/v4/soroban_meta/return_value",
    "byte_range": { "offset": 1784, "length": 48 }
  },
  "sightings": ["https://github.com/stellar/js-stellar-sdk/issues/1789"]
}
```

| Field              | Meaning                                                                                                                            |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| `id`               | The name of the entry.                                                                                                             |
| `description`      | The fact that a reader of the shape must keep.                                                                                     |
| `caps`             | The CAPs that introduced the shape, ascending. The list is empty for a shape that the network had at launch.                       |
| `since_protocol`   | The first protocol under which the network can emit the shape, 0 for a shape that the network had at launch.                       |
| `xdr_type`         | The XDR type of the shape.                                                                                                         |
| `soroban_validity` | What `soroban-env-host` answers about the value: `valid` or `invalid`. `not_applicable` marks a value that is not a Soroban value. |
| `location`         | The place of the shape: the `source`, the `json_pointer` into its decoded value, and the `byte_range` in its bytes.                |
| `sightings`        | The upstream issues and pull requests where the shape broke a reader.                                                              |

## Contents

The corpus holds seven entries.

| Entry                           | Shape                                                                                          | Origin       |
| ------------------------------- | ---------------------------------------------------------------------------------------------- | ------------ |
| `scval-map-colliding-keys`      | A Soroban map whose keys `1` and `"1"` print the same.                                         | stellar-core |
| `scval-map-unsorted-keys`       | A Soroban map with keys out of order, which the Soroban host refuses.                          | stellar-core |
| `scval-string-non-utf8`         | A Soroban string with the byte `ff`, which is not UTF-8.                                       | stellar-core |
| `memo-text-non-utf8`            | A text memo with the byte `ff`, which is not UTF-8.                                            | stellar-core |
| `transaction-no-preconditions`  | A transaction with no precondition, not even time bounds. It stands in the ledger of the memo. | stellar-core |
| `operation-offer-id-above-2-53` | An operation that names the offer 2^53 + 1, which a 64-bit float cannot hold.                  | stellar-core |
| `stellar-value-empty-tx-set`    | A ledger value whose transaction set the validators voted to drop.                             | synthetic    |

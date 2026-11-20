# JS runner

The runner reads the corpus through `@stellar/stellar-sdk` and reports a verdict for each check.

```sh
(cd runners/js && npm install && npm start)
```

## Checks

A check runs the SDK on one source or one entry and compares the result with the corpus. The checks stand on three levels.

```
codec    every type
json     every type
helper   ScVal        scValToNative, union.value, contract.Spec
         Memo         union.value, Memo.value
         Operation    rebuild, rebuild(numbers)
         Transaction  cloneFrom
```

`codec` and `json` need no code for one type:

- `codec` reads the bytes into an SDK object and writes them back.
- `json` writes the object as XDR-JSON, compares it with the decoded value of the corpus, and reads that value back into bytes.

A helper is tied to the XDR type of an entry. It exercises the functions of the SDK that an application calls for values of that type:

- `scValToNative` reads a Soroban value into a value of the language and compares the whole result with the decoded value.
- `union.value` rebuilds a generated union from the name of its arm and its `value`. It serves every union type.
- `contract.Spec` reads a Soroban value by its spec type and builds it again, as a generated contract client does. The type comes from the value, so the helper serves every Soroban value of one type. It skips a vector and a map, which may hold values of several types.
- `Memo.value` reads a memo into the class `Memo` and builds it again.
- `rebuild` lets the SDK read an operation into its fields and build it again with the builder of that kind. It serves every kind of operation.
- `rebuild(numbers)` does the same and passes every integer field as a JavaScript number, as an application may.
- `cloneFrom` lets the SDK read a transaction, clone it and build it again, as `rpc.assembleTransaction` does.

The corpus holds the value, and the runner holds the forms in which JavaScript passes it.

## Verdicts

| Verdict   | Meaning                                       | Run    |
| --------- | --------------------------------------------- | ------ |
| `kept`    | The SDK kept the value.                       | passes |
| `lost`    | The SDK returned another value with no error. | fails  |
| `refused` | The SDK raised an error.                      | fails  |
| `skipped` | The check has no rule for this value.         | passes |

## Known verdicts

The team of an SDK may know and accept some verdicts: a defect with an open issue, or a refusal that fits this SDK. The team keeps them in a file and passes it to the runner.

```sh
(cd runners/js && npm start -- --known known.json)
```

```json
[
  {
    "id": "memo-text-non-utf8",
    "check": "union.value",
    "verdict": "lost",
    "reason": "union.value reads the memo as text, with U+FFFD in place of the byte (issue https://github.com/stellar/js-stellar-sdk/issues/1823)"
  }
]
```

| Verdict of a check    | In the file              | Run                                                                |
| --------------------- | ------------------------ | ------------------------------------------------------------------ |
| `lost` or `refused`   | listed with this verdict | passes, and the report shows the reason                            |
| `lost` or `refused`   | not listed               | fails                                                              |
| any other than listed | listed                   | fails: the SDK changed, and the item must change or leave the file |

The file belongs to the repository of the SDK, because each release has its own list. The file [`known.json`](known.json) of this directory is the list of the installed release, and CI runs the runner with it. The [reproductions](../../reproductions) pass one such file for each release that they run.

## A run

On release 17.4.0, with no file of known verdicts, the run reports three losses and three refusals. Two losses are the memo and the Soroban string of [js-stellar-sdk#1823](https://github.com/stellar/js-stellar-sdk/issues/1823), and the third is the same string read through `contract.Spec`. Two refusals come from the fixes of #1789 and #1785, and the third is the transaction with no preconditions, which `cloneFrom` does not build.

<details>
<summary>The report</summary>

```
@stellar/stellar-sdk 17.4.0, no known verdicts

kept             ledger-contract-call           codec
kept             ledger-contract-call           json
kept             ledger-contract-refusal        codec
kept             ledger-contract-refusal        json
kept             ledger-contract-string         codec
kept             ledger-contract-string         json
kept             ledger-memo-text               codec
kept             ledger-memo-text               json
kept             ledger-offer-update            codec
kept             ledger-offer-update            json
kept             value-empty-tx-set             codec
kept             value-empty-tx-set             json
kept             scval-map-colliding-keys       codec
kept             scval-map-colliding-keys       json
refused          scval-map-colliding-keys       scValToNative     TypeError: scvMap has more than one key that converts to "1"; read the raw map entries instead
kept             scval-map-colliding-keys       union.value
skipped          scval-map-colliding-keys       contract.Spec     the runner gives no spec type to a vector or a map, whose values can have several types
kept             scval-map-unsorted-keys        codec
kept             scval-map-unsorted-keys        json
kept             scval-map-unsorted-keys        scValToNative
kept             scval-map-unsorted-keys        union.value
skipped          scval-map-unsorted-keys        contract.Spec     the runner gives no spec type to a vector or a map, whose values can have several types
kept             scval-string-non-utf8          codec
kept             scval-string-non-utf8          json
kept             scval-string-non-utf8          scValToNative
lost             scval-string-non-utf8          union.value       the rebuilt value has other bytes
lost             scval-string-non-utf8          contract.Spec     the rebuilt value has other bytes
kept             memo-text-non-utf8             codec
kept             memo-text-non-utf8             json
lost             memo-text-non-utf8             union.value       the rebuilt value has other bytes
kept             memo-text-non-utf8             Memo.value
kept             transaction-no-preconditions   codec
kept             transaction-no-preconditions   json
refused          transaction-no-preconditions   cloneFrom         Error: TimeBounds has to be set or you must call setTimeout(TimeoutInfinite).
kept             operation-offer-id-above-2-53  codec
kept             operation-offer-id-above-2-53  json
kept             operation-offer-id-above-2-53  rebuild
refused          operation-offer-id-above-2-53  rebuild(numbers)  Error: offerId must be a non-negative safe integer; pass a larger ID as a bigint or a decimal string
kept             stellar-value-empty-tx-set     codec
kept             stellar-value-empty-tx-set     json

40 checks: 32 kept, 3 lost, 3 refused, 2 skipped

6 failures: each lost or refused line without (known), each line that states another known verdict, and each fail line
```

</details>

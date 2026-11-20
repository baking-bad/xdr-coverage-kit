# Reproductions

A reproduction runs the JS runner on the releases of an SDK around a fix. Each release has a file of its known verdicts, and the run fails when a release gives another verdict. So a reproduction is a check, and CI runs it.

## js-stellar-sdk-17.3.0

Release 17.3.0 of `@stellar/stellar-sdk` fixes two defects, and 17.2.1 is the last release before it.

| Defect                                                         | Entry                           | Check              | 17.2.1 | 17.3.0  |
| -------------------------------------------------------------- | ------------------------------- | ------------------ | ------ | ------- |
| [#1789](https://github.com/stellar/js-stellar-sdk/issues/1789) | `scval-map-colliding-keys`      | `scValToNative`    | lost   | refused |
| [#1785](https://github.com/stellar/js-stellar-sdk/issues/1785) | `operation-offer-id-above-2-53` | `rebuild(numbers)` | lost   | refused |

```sh
(cd runners/js && npm install)
(cd reproductions/js-stellar-sdk-17.3.0 && npm install && npm start)
```

<details>
<summary>The report on 17.2.1</summary>

```
@stellar/stellar-sdk 17.2.1, known verdicts from known-17.2.1.json

refused (known)  ledger-contract-call           codec             17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-contract-call           json              17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-contract-refusal        codec             17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-contract-refusal        json              17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-contract-string         codec             17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-contract-string         json              17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-memo-text               codec             17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-memo-text               json              17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-offer-update            codec             17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  ledger-offer-update            json              17.2.1 is older than protocol 30 and cannot decode the ledger. SDK: XdrError: LedgerCloseMeta.v2.v2.ledgerHeader.header.scpValue.ext.v: unknown enum value 3
refused (known)  value-empty-tx-set             codec             17.2.1 is older than protocol 30 and cannot decode the ledger value. SDK: XdrError: StellarValue.ext.v: unknown enum value 4
refused (known)  value-empty-tx-set             json              17.2.1 is older than protocol 30 and cannot decode the ledger value. SDK: XdrError: StellarValue.ext.v: unknown enum value 4
kept             scval-map-colliding-keys       codec
kept             scval-map-colliding-keys       json
lost (known)     scval-map-colliding-keys       scValToNative     scValToNative drops one of two keys that print the same (issue https://github.com/stellar/js-stellar-sdk/issues/1789)
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
lost (known)     scval-string-non-utf8          union.value       union.value reads the string as text, with U+FFFD in place of the byte (issue https://github.com/stellar/js-stellar-sdk/issues/1823)
lost (known)     scval-string-non-utf8          contract.Spec     contract.Spec reads the string as text, with U+FFFD in place of the byte (related issue https://github.com/stellar/js-stellar-sdk/issues/1823)
kept             memo-text-non-utf8             codec
kept             memo-text-non-utf8             json
lost (known)     memo-text-non-utf8             union.value       union.value reads the memo as text, with U+FFFD in place of the byte (issue https://github.com/stellar/js-stellar-sdk/issues/1823)
kept             memo-text-non-utf8             Memo.value
kept             transaction-no-preconditions   codec
kept             transaction-no-preconditions   json
refused (known)  transaction-no-preconditions   cloneFrom         cloneFrom requires time bounds, and the transaction states no precondition (related issue https://github.com/stellar/js-stellar-sdk/issues/1911). SDK: Error: TimeBounds has to be set or you must call setTimeout(TimeoutInfinite).
kept             operation-offer-id-above-2-53  codec
kept             operation-offer-id-above-2-53  json
kept             operation-offer-id-above-2-53  rebuild
lost (known)     operation-offer-id-above-2-53  rebuild(numbers)  Operation.manageSellOffer accepts an offer ID above 2^53 as a number, which holds another ID (issue https://github.com/stellar/js-stellar-sdk/issues/1785)
refused (known)  stellar-value-empty-tx-set     codec             17.2.1 is older than protocol 30 and cannot decode the ledger value. SDK: XdrError: StellarValue.ext.v: unknown enum value 4
refused (known)  stellar-value-empty-tx-set     json              17.2.1 is older than protocol 30 and cannot decode the ledger value. SDK: XdrError: StellarValue.ext.v: unknown enum value 4

40 checks: 18 kept, 5 lost (known), 15 refused (known), 2 skipped
```

</details>

On 17.3.0 the report gives the same verdicts as [the run of the runner](../runners/js#a-run) on 17.4.0, and the file of known verdicts names each loss and refusal.

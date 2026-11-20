# Contracts

This directory holds the Soroban contracts that the scenarios deploy. Each contract has a directory with its source, and [`wasm/`](wasm) holds the built Wasm of every contract, so the generator needs no contract build.

| Contract       | What it does                                                          | Scenarios                                              |
| -------------- | --------------------------------------------------------------------- | ------------------------------------------------------ |
| [`echo`](echo) | Its function `echo` takes a map, emits it in an event and returns it. | `contract_call`, `contract_refusal`, `contract_string` |

`stellar contract build --out-dir ../wasm` in the directory of a contract builds its Wasm and writes it to `wasm/`. It needs the Stellar CLI 30.

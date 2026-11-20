//! A contract that returns its argument and emits it in an event.

#![no_std]

use soroban_sdk::{Env, Map, Val, contract, contractevent, contractimpl};

#[contract]
pub struct Echo;

/// The event of a call: its one topic is `echo` and its data is the map.
#[contractevent(topics = ["echo"], data_format = "single-value")]
pub struct Echoed {
    pub map: Map<Val, Val>,
}

#[contractimpl]
impl Echo {
    /// Emits `map` in an [`Echoed`] event and returns it.
    pub fn echo(env: Env, map: Map<Val, Val>) -> Map<Val, Val> {
        Echoed { map: map.clone() }.publish(&env);
        map
    }
}

//! XDR values by type name, as bytes and as XDR-JSON, and what the Soroban
//! host answers about them.

use crate::manifest::SorobanValidity;
use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use soroban_env_host::{Host, TryFromVal, Val};
use std::str::FromStr;
use stellar_xdr::{Limits, ScVal, Type, TypeVariant, WriteXdr};

/// The limits of every XDR read and write.
// NOTE: 500 is the depth limit of the Soroban host (`DEFAULT_XDR_RW_LIMITS`),
// which the Rust clients of Stellar also use.
pub const LIMITS: Limits = Limits {
    depth: 500,
    len: usize::MAX,
};

pub fn variant(name: &str) -> Result<TypeVariant> {
    TypeVariant::from_str(name)
        .map_err(|_| anyhow!("stellar-xdr has no type {name}"))
}

pub fn from_bytes(variant: TypeVariant, bytes: &[u8]) -> Result<Type> {
    Type::from_xdr(variant, bytes, LIMITS).map_err(|error| {
        anyhow!("the bytes do not decode as {}: {error}", variant.name())
    })
}

pub fn from_json(variant: TypeVariant, json: &Value) -> Result<Type> {
    // NOTE: stellar-xdr reads JSON by type name only from text.
    Type::from_json(variant, json.to_string().as_bytes()).with_context(|| {
        format!("the JSON does not decode as {}", variant.name())
    })
}

pub fn to_bytes(value: &Type) -> Result<Vec<u8>> {
    Ok(value.to_xdr(LIMITS)?)
}

pub fn to_json(value: &Type) -> Result<Value> {
    Ok(serde_json::to_value(value)?)
}

/// The answer of the Soroban host about a value. The host converts the value
/// as it converts the argument of a contract call. It refuses, for example,
/// a map whose keys are out of order. A value that is neither an `ScVal` nor
/// an `ScMap` gives `NotApplicable`.
pub fn soroban_validity(value: &Type) -> SorobanValidity {
    let value = match value {
        Type::ScVal(value) => value.as_ref().clone(),
        Type::ScMap(map) => ScVal::Map(Some(map.as_ref().clone())),
        _ => return SorobanValidity::NotApplicable,
    };
    match Val::try_from_val(&Host::default(), &value) {
        Ok(_) => SorobanValidity::Valid,
        Err(_) => SorobanValidity::Invalid,
    }
}

/// Appends to `found` the JSON Pointer of every place where `value` holds
/// `part`, in the order of the JSON. `at` is the pointer of `value` itself.
/// The search does not look inside a match.
// NOTE: a key of XDR-JSON is the name of an XDR field or of a union arm.
// Neither holds `/` or `~`, so no key needs the escape of JSON Pointer.
pub fn pointers_to(
    value: &Value,
    part: &Value,
    at: String,
    found: &mut Vec<String>,
) {
    if value == part {
        found.push(at);
        return;
    }
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                pointers_to(item, part, format!("{at}/{index}"), found);
            }
        }
        Value::Object(fields) => {
            for (key, field) in fields {
                pointers_to(field, part, format!("{at}/{key}"), found);
            }
        }
        _ => {}
    }
}

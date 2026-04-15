mod cache;
mod callsite;

use std::{ffi::CStr, sync::Arc};

use pgrx::*;
use serde_json::Value;

pub(crate) use callsite::fn_extra_get_or_compile;

/// JSON schema is stored as its canonical JSON string.
///
/// Canonicalization ensures semantically equivalent schemas share one string
/// representation, maximising cache hits. The compiled [`jsonschema::Validator`]
/// is held in a two-level cache: a per-callsite slot in `fn_extra`  and a
/// bounded backend-local LRU.
#[derive(
    PostgresType,
    PostgresEq,
    PostgresHash,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[inoutfuncs]
pub struct JsonSchema {
    pub value: String,
}

impl JsonSchema {
    /// Canonicalize, compile, and cache a JSON schema value.
    pub(crate) fn compile(value: Value) -> Self {
        let canonical = jsonschema::canonical::json::to_string(&value)
            .unwrap_or_else(|err| pgrx::error!("failed to canonicalize JSON schema: {err}"));
        cache::get_or_insert(&canonical, || compile_impl(&value, "invalid JSON schema"));
        Self { value: canonical }
    }
}

impl pgrx::inoutfuncs::InOutFuncs for JsonSchema {
    fn input(input: &CStr) -> Self {
        let value: Value = serde_json::from_slice(input.to_bytes())
            .unwrap_or_else(|err| pgrx::error!("invalid JSON: {err}"));
        Self::compile(value)
    }

    fn output(&self, buffer: &mut pgrx::StringInfo) {
        buffer.push_str(&self.value);
    }
}

fn compile_impl(value: &Value, error_prefix: &str) -> Arc<jsonschema::Validator> {
    Arc::new(
        jsonschema::validator_for(value)
            .unwrap_or_else(|err| pgrx::error!("{error_prefix}: {err}")),
    )
}

pub(super) fn compile_from_str(schema: &str) -> Arc<jsonschema::Validator> {
    let value: Value = serde_json::from_str(schema)
        .unwrap_or_else(|err| pgrx::error!("internal: failed to parse canonical schema: {err}"));
    compile_impl(&value, "internal: failed to compile schema")
}

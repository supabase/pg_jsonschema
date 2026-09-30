use std::sync::Arc;

use pgrx::*;

use super::{JsonSchema, cache, compile_from_str};

fn get_or_compile(schema: &JsonSchema) -> Arc<jsonschema::Validator> {
    cache::get_or_insert(&schema.value, || compile_from_str(&schema.value))
}

/// Per-callsite validator cache in `fcinfo->flinfo->fn_extra`.
struct FnExtraCache {
    schema: String,
    validator: Arc<jsonschema::Validator>,
    info: *mut pg_sys::FmgrInfo,
    const_schema_arg: bool,
    /// MemoryContextCallback; fires `drop_fn_extra_cache` when fn_mcxt is reset.
    callback: pg_sys::MemoryContextCallback,
}

unsafe extern "C-unwind" fn drop_fn_extra_cache(arg: *mut std::ffi::c_void) {
    // PG calls callbacks before freeing memory, so `entry` and `(*entry).flinfo` are
    // still valid. Null fn_extra first so any re-entrant drop sees a clean slate.
    unsafe {
        let entry = arg as *mut FnExtraCache;
        (*(*entry).info).fn_extra = std::ptr::null_mut();
        std::ptr::drop_in_place(entry);
    }
}

/// `get_fn_expr_arg_stable` also accepts `PARAM_EXTERN` params, whose value changes between
/// calls sharing one `FmgrInfo` (e.g. a PL/pgSQL variable in a loop).
unsafe fn schema_arg_is_const(flinfo: *mut pg_sys::FmgrInfo) -> bool {
    unsafe {
        let expr = (*flinfo).fn_expr;
        if !is_a(expr, pg_sys::NodeTag::T_FuncExpr) {
            return false;
        }
        let args = PgList::<pg_sys::Node>::from_pg((*expr.cast::<pg_sys::FuncExpr>()).args);
        args.get_ptr(0)
            .is_some_and(|arg| is_a(arg, pg_sys::NodeTag::T_Const))
    }
}

/// Returns a compiled validator for `schema`, using a two-level cache.
///
/// **L1** — per-callsite slot in `fcinfo->flinfo->fn_extra` (lifetime: `fn_mcxt`).
/// When the schema argument is a constant, the slot is reused unconditionally; otherwise it
/// is reused on a string match.
///
/// **L2** — backend-local LRU (see [`super::cache`]).  Hit on L1 miss.
///
/// # Why two levels?
///
/// The L2 LRU lookup hashes the full canonical schema string on every call.
/// For small schemas this is negligible, but large schemas make it expensive:
/// a 3.3 MB FHIR schema hashed across 20k rows takes ~13 s via LRU alone vs
/// ~1.8 s with the L1 callsite cache, which reduces each hot-path lookup to a
/// pointer dereference.
///
/// # Safety
/// `fcinfo` must be a valid, non-null `FunctionCallInfo` for the current call.
pub(crate) unsafe fn fn_extra_get_or_compile(
    schema: &JsonSchema,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Arc<jsonschema::Validator> {
    unsafe {
        let flinfo = (*fcinfo).flinfo;
        let cached_ptr = if !(*flinfo).fn_extra.is_null() {
            Some((*flinfo).fn_extra as *mut FnExtraCache)
        } else {
            None
        };

        // L1 hit: schema matches (or arg is a constant, so it can't change).
        if let Some(cached) = cached_ptr.map(|ptr| &*ptr)
            && (cached.const_schema_arg || cached.schema.as_str() == schema.value.as_str())
        {
            return Arc::clone(&cached.validator);
        }

        // L1 miss: pay the constness check once, then refresh or allocate.
        let const_schema_arg = schema_arg_is_const(flinfo);

        // Cache miss: refresh the callsite entry.
        if let Some(cached_ptr) = cached_ptr {
            let cached = &mut *cached_ptr;
            let validator = get_or_compile(schema);
            let next = cached.callback.next;
            let old_entry = std::mem::replace(
                cached,
                FnExtraCache {
                    schema: schema.value.clone(),
                    validator: Arc::clone(&validator),
                    info: flinfo,
                    const_schema_arg,
                    callback: pg_sys::MemoryContextCallback {
                        func: Some(drop_fn_extra_cache),
                        arg: cached_ptr as *mut std::ffi::c_void,
                        next,
                    },
                },
            );
            drop(old_entry);
            return validator;
        }

        // Cold path: allocate in fn_mcxt.
        let validator = get_or_compile(schema);
        let fn_mcxt = (*flinfo).fn_mcxt;
        let old_mcxt = pg_sys::MemoryContextSwitchTo(fn_mcxt);

        let cache_ptr = pg_sys::palloc(std::mem::size_of::<FnExtraCache>()) as *mut FnExtraCache;
        std::ptr::write(
            cache_ptr,
            FnExtraCache {
                schema: schema.value.clone(),
                validator: Arc::clone(&validator),
                info: flinfo,
                const_schema_arg,
                callback: pg_sys::MemoryContextCallback {
                    func: Some(drop_fn_extra_cache),
                    arg: cache_ptr as *mut std::ffi::c_void,
                    next: std::ptr::null_mut(),
                },
            },
        );
        (*flinfo).fn_extra = cache_ptr as *mut std::ffi::c_void;
        pg_sys::MemoryContextRegisterResetCallback(
            fn_mcxt,
            std::ptr::addr_of_mut!((*cache_ptr).callback),
        );
        pg_sys::MemoryContextSwitchTo(old_mcxt);

        validator
    }
}

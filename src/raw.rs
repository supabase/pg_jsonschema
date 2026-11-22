use jsonschema::json::{JsonText, JsonTextDocument};
use pgrx::{FromDatum, error, pg_sys};

/// A `jsonb` argument, detoasted but not deserialized.
pub struct RawJsonb(*mut pg_sys::varlena);

impl RawJsonb {
    /// The `JsonbContainer` bytes, valid for the current call.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { pgrx::varlena_to_byte_slice(self.0) }
    }
}

impl FromDatum for RawJsonb {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        _typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            return None;
        }
        // `_packed` keeps a short header instead of copying; `varlena_to_byte_slice` reads both.
        Some(RawJsonb(unsafe {
            pg_sys::pg_detoast_datum_packed(datum.cast_mut_ptr())
        }))
    }
}

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for RawJsonb {
    unsafe fn unbox_arg_unchecked(arg: pgrx::callconv::Arg<'_, 'fcx>) -> Self {
        let index = arg.index();
        unsafe { arg.unbox_arg_using_from_datum() }
            .unwrap_or_else(|| panic!("argument {index} must not be null"))
    }
}

pgrx::impl_sql_translatable!(RawJsonb, "jsonb");

/// A `json` argument, detoasted but not parsed.
pub struct RawJson(*mut pg_sys::varlena);

impl RawJson {
    /// The checked text, valid for the current call. Text that `json` stores but strict parsing
    /// rejects, such as nesting past 128 levels, raises the same error as a parsed `json` schema
    /// argument.
    pub fn parse(&self) -> JsonTextDocument<'_> {
        let bytes = unsafe { pgrx::varlena_to_byte_slice(self.0) };
        JsonText::parse(bytes)
            .unwrap_or_else(|err| error!("datum must refer to a valid json varlena: {err:?}"))
    }
}

impl FromDatum for RawJson {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        _typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            return None;
        }
        Some(RawJson(unsafe {
            pg_sys::pg_detoast_datum_packed(datum.cast_mut_ptr())
        }))
    }
}

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for RawJson {
    unsafe fn unbox_arg_unchecked(arg: pgrx::callconv::Arg<'_, 'fcx>) -> Self {
        let index = arg.index();
        unsafe { arg.unbox_arg_using_from_datum() }
            .unwrap_or_else(|| panic!("argument {index} must not be null"))
    }
}

pgrx::impl_sql_translatable!(RawJson, "json");

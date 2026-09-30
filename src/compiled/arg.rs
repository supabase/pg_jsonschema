use pgrx::{FromDatum, pg_sys};

use super::JsonSchema;

/// A `jsonschema` argument as stored, decoded only on demand.
pub struct SchemaArg(pg_sys::Datum);

impl SchemaArg {
    /// The datum's bytes as stored: a TOAST pointer or the inline value. Stored values are
    /// immutable, so equal bytes mean an equal schema.
    pub fn stored_bytes(&self) -> &[u8] {
        let ptr = self.0.cast_mut_ptr::<pg_sys::varlena>();
        unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), pgrx::varsize_any(ptr)) }
    }

    pub fn decode(&self) -> JsonSchema {
        unsafe { JsonSchema::from_datum(self.0, false) }.expect("a strict function has no null")
    }
}

impl FromDatum for SchemaArg {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        _typoid: pg_sys::Oid,
    ) -> Option<Self> {
        (!is_null).then_some(SchemaArg(datum))
    }
}

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for SchemaArg {
    unsafe fn unbox_arg_unchecked(arg: pgrx::callconv::Arg<'_, 'fcx>) -> Self {
        let index = arg.index();
        unsafe { arg.unbox_arg_using_from_datum() }
            .unwrap_or_else(|| panic!("argument {index} must not be null"))
    }
}

pgrx::impl_sql_translatable!(SchemaArg, "jsonschema");

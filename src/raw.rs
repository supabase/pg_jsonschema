use pgrx::{FromDatum, pg_sys};

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

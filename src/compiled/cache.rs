/// Backend-local LRU caches mapping canonical schema strings to compiled validators, one per
/// instance representation.
///
/// PostgreSQL backends are single-threaded OS processes, so a `thread_local`
/// `RefCell` is sufficient — no mutex needed.
use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::thread::LocalKey;

use jsonschema::{
    Validator,
    json::{Json, Jsonb, SerdeJson},
};

type Cache<F> = lru::LruCache<String, Arc<Validator<F>>>;

const CAPACITY: NonZeroUsize = NonZeroUsize::new(128).expect("128 is non zero");

thread_local! {
    static SERDE_JSON: RefCell<Cache<SerdeJson>> = RefCell::new(lru::LruCache::new(CAPACITY));
    static JSONB: RefCell<Cache<Jsonb>> = RefCell::new(lru::LruCache::new(CAPACITY));
}

/// An instance representation with its own validator cache.
pub(crate) trait Cached: Json + Sized {
    fn cache() -> &'static LocalKey<RefCell<Cache<Self>>>;
}

impl Cached for SerdeJson {
    fn cache() -> &'static LocalKey<RefCell<Cache<Self>>> {
        &SERDE_JSON
    }
}

impl Cached for Jsonb {
    fn cache() -> &'static LocalKey<RefCell<Cache<Self>>> {
        &JSONB
    }
}

/// Returns the cached validator for `schema`, inserting one produced by `f` on a miss.
pub(super) fn get_or_insert<F: Cached>(
    schema: &str,
    f: impl FnOnce() -> Arc<Validator<F>>,
) -> Arc<Validator<F>> {
    F::cache().with_borrow_mut(|c| {
        if let Some(v) = c.get(schema) {
            return Arc::clone(v);
        }
        let validator = f();
        c.put(schema.to_owned(), Arc::clone(&validator));
        validator
    })
}

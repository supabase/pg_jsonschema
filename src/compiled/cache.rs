/// Backend-local LRU cache mapping canonical schema strings to compiled validators.
///
/// PostgreSQL backends are single-threaded OS processes, so a `thread_local`
/// `RefCell` is sufficient — no mutex needed.
use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::sync::Arc;

type Cache = lru::LruCache<String, Arc<jsonschema::Validator>>;

const CAPACITY: NonZeroUsize = NonZeroUsize::new(128).expect("128 is non zero");

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(lru::LruCache::new(CAPACITY));
}

/// Returns the cached validator for `schema`, inserting one produced by `f` on a miss.
pub(super) fn get_or_insert(
    schema: &str,
    f: impl FnOnce() -> Arc<jsonschema::Validator>,
) -> Arc<jsonschema::Validator> {
    CACHE.with_borrow_mut(|c| {
        if let Some(v) = c.get(schema) {
            return Arc::clone(v);
        }
        let validator = f();
        c.put(schema.to_owned(), Arc::clone(&validator));
        validator
    })
}

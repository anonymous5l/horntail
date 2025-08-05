//! Simply access resource data helper.
//!
//! NOTICE: All operation on `Entry` will trigger `IO`.
//!
//! To minimize IO overhead, consider use `EntryCache`, it can be converted from `Entry`.
//!
//! ## Example
//!
//! ```no_run
//! use horntail::crypto::{MapleCipher, MapleTableNone, MapleVersion};
//! use horntail::extra::Entry;
//!
//! let entry = Entry::from_path("Base", MapleTableNone.into_boxed(), MapleVersion::from(217), false).unwrap();
//!
//! let _ = entry.get("Character");
//!
//! // use cache to minimize `IO` operations.
//! let cache = entry.into_cache();
//! let _ = cache.get("Character");
//! ```
//!
mod bundle;
mod cache;
mod entry;
mod entry_ext;
mod iter;

pub use cache::EntryCache;
pub use entry::{Entry, EntryPrimitive, EntryValue};

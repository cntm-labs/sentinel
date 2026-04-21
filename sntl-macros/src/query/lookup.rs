//! Workspace-root discovery + cache reads for proc-macro-time SQL validation.

use proc_macro2::Span;
use proc_macro_error2::abort;
use sntl_schema::cache::{Cache, CacheEntry};
use sntl_schema::normalize::hash_sql;
use sntl_schema::schema::Schema;
use std::path::PathBuf;

/// Walk up from CARGO_MANIFEST_DIR until a `sentinel.toml` or `.sentinel/`
/// directory is found. Falls back to the current directory.
pub fn workspace_root() -> PathBuf {
    let mut cur: PathBuf = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        if cur.join("sentinel.toml").exists() || cur.join(".sentinel").exists() {
            return cur;
        }
        if !cur.pop() {
            return PathBuf::from(".");
        }
    }
}

pub fn open_cache() -> Cache {
    let root = workspace_root();
    Cache::new(root.join(".sentinel"))
}

pub fn load_schema(span: Span) -> Schema {
    let cache = open_cache();
    match cache.read_schema() {
        Ok(s) => s,
        Err(e) => abort!(span, "cannot read schema snapshot: {}", e;
            help = "run `sntl prepare` to generate .sentinel/schema.toml"),
    }
}

pub fn lookup_entry(sql: &str, span: Span) -> CacheEntry {
    let hash = hash_sql(sql);
    let cache = open_cache();
    match cache.read_entry(&hash) {
        Ok(e) => e,
        Err(e) => abort!(span, "query not found in cache (.sentinel/queries/{}.json): {}", hash, e;
            help = "run `sntl prepare` with DB connection, then commit .sentinel/";
            help = "or use `sntl::query_unchecked!` to skip validation temporarily"),
    }
}

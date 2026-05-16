//! Internal, NOT part of the public API. Used by `sntl-macros` only.
#![doc(hidden)]

use driver::Event;

/// Emit a `QueryMacro` event onto the connection's instrumentation.
///
/// `query_id` should be the 13-char hash from `.sentinel/queries/<id>.json`
/// so consumers can correlate to the offline cache entry.
pub fn emit_query_macro(conn: &driver::Connection, macro_name: &str, query_id: &str, sql: &str) {
    conn.instrumentation().on_event(&Event::QueryMacro {
        macro_name,
        query_id,
        sql,
    });
}

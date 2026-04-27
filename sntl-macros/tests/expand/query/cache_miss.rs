// Compile-fail: SQL not in .sentinel/ cache should abort with a clear
// "query not found in cache" diagnostic.

fn main() {
    let _ = sntl::query!("SELECT no_such_query_ever");
}

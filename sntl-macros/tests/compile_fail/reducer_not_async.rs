//! `#[sntl::reducer]` on a sync fn is rejected.

use sntl::driver::Connection;

#[sntl::reducer]
fn not_async(conn: &mut Connection) -> sntl::Result<()> {
    let _ = conn;
    Ok(())
}

fn main() {}

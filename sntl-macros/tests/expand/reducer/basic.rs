//! Default form (no isolation arg) compiles and produces an async fn.

use sntl::driver::Connection;

#[sntl::reducer]
async fn transfer(conn: &mut Connection, _from: i32, _to: i32, _amount: i64) -> sntl::Result<()> {
    let _ = conn;
    Ok(())
}

fn main() {
    // Compile-time only — fn is not called.
    let _ = transfer;
}

//! Invalid isolation string is rejected at expand time.

use sntl::driver::Connection;

#[sntl::reducer(isolation = "snapshot")]
async fn bad(conn: &mut Connection) -> sntl::Result<()> {
    let _ = conn;
    Ok(())
}

fn main() {}

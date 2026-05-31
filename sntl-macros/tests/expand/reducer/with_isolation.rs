//! `isolation = "serializable"` arg compiles.

use sntl::driver::Connection;

#[sntl::reducer(isolation = "serializable")]
async fn cas(conn: &mut Connection, _id: i32) -> sntl::Result<i64> {
    let _ = conn;
    Ok(0)
}

fn main() {
    let _ = cas;
}

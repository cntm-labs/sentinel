//! First arg must be `&mut driver::Connection`.

#[sntl::reducer]
async fn missing_conn(other: i32) -> sntl::Result<()> {
    let _ = other;
    Ok(())
}

fn main() {}

// Compile-fail: tuple expects 2 columns but the SELECT returns only 1.
fn main() {
    let id: i32 = 1;
    let _ = sntl::query_as!(
        (i32, String),
        "SELECT id FROM users WHERE id = $1",
        id
    );
}

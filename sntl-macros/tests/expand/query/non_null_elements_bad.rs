// Compile-fail: non_null_elements references a non-array column.
fn main() {
    let _ = sntl::query!(
        "SELECT id FROM users WHERE id = $1",
        1i32,
        non_null_elements = [id]
    );
}

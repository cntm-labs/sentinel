//! Tiny helper that prints `hash_sql(arg)` so trybuild fixtures can know
//! what cache file to seed for a given SQL string.
//!
//! Usage:
//! ```bash
//! cargo run -p sntl-schema --example compute_hash --quiet -- "SELECT id FROM users WHERE id = $1"
//! ```

fn main() {
    let sql = std::env::args()
        .nth(1)
        .expect("usage: compute_hash '<sql>'");
    println!("{}", sntl_schema::normalize::hash_sql(&sql));
}

use sntl_schema::normalize::{hash_sql, normalize_sql};

#[test]
fn strips_line_comments() {
    let got = normalize_sql("SELECT id -- comment\nFROM users");
    assert_eq!(got, "SELECT id FROM users");
}

#[test]
fn strips_block_comments() {
    let got = normalize_sql("SELECT /* block\n comment */ id FROM users");
    assert_eq!(got, "SELECT id FROM users");
}

#[test]
fn collapses_whitespace() {
    let got = normalize_sql("SELECT    id\n\n\tFROM  users");
    assert_eq!(got, "SELECT id FROM users");
}

#[test]
fn preserves_string_literal_contents() {
    let got = normalize_sql("SELECT 'hello   world -- not a comment' FROM t");
    assert_eq!(got, "SELECT 'hello   world -- not a comment' FROM t");
}

#[test]
fn identical_sql_hashes_identically() {
    let a = hash_sql("SELECT id FROM users WHERE id = $1");
    let b = hash_sql("SELECT  id\nFROM  users\nWHERE id = $1");
    assert_eq!(a, b);
}

#[test]
fn different_sql_hashes_differently() {
    let a = hash_sql("SELECT id FROM users");
    let b = hash_sql("SELECT id FROM posts");
    assert_ne!(a, b);
}

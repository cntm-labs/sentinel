use sntl_migrate::checksum::sha256_of_sql;

#[test]
fn deterministic() {
    let a = sha256_of_sql("CREATE TABLE foo (id int);");
    let b = sha256_of_sql("CREATE TABLE foo (id int);");
    assert_eq!(a, b);
}

#[test]
fn sensitive_to_whitespace() {
    let a = sha256_of_sql("CREATE TABLE foo (id int);");
    let b = sha256_of_sql("CREATE  TABLE foo (id int);");
    assert_ne!(a, b, "whitespace difference must change the hash");
}

#[test]
fn truncated_length_is_13() {
    let h = sha256_of_sql("anything");
    assert_eq!(h.len(), 13);
}

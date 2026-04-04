use sentinel_core::Error;

#[test]
fn error_display_column_not_found() {
    let err = Error::ColumnNotFound {
        column: "email".into(),
        table: "users".into(),
    };
    assert_eq!(err.to_string(), "column 'email' not found in table 'users'");
}

#[test]
fn error_display_query_build() {
    let err = Error::QueryBuild("missing WHERE clause".into());
    assert_eq!(err.to_string(), "query build error: missing WHERE clause");
}

#[test]
fn error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Error>();
}

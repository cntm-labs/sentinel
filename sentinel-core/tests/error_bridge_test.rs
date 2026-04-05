use sentinel_core::error::Error;

#[test]
fn driver_error_converts_to_sentinel_error() {
    let driver_err = sentinel_driver::Error::Protocol("test protocol error".into());
    let sentinel_err: Error = driver_err.into();
    assert!(matches!(sentinel_err, Error::Driver(_)));
    assert!(sentinel_err.to_string().contains("test protocol error"));
}

#[test]
fn not_found_error_from_driver() {
    let driver_err = sentinel_driver::Error::Protocol("query returned no rows".into());
    let sentinel_err: Error = driver_err.into();
    assert!(matches!(sentinel_err, Error::Driver(_)));
}

use sntl_migrate::migration::Version;

#[test]
fn parses_valid_folder_name() {
    let v: Version = "20260509_140000_add_users".parse().unwrap();
    assert_eq!(v.timestamp(), "20260509_140000");
    assert_eq!(v.name(), "add_users");
    assert_eq!(v.as_str(), "20260509_140000_add_users");
}

#[test]
fn rejects_short_timestamp() {
    assert!("2026_add_users".parse::<Version>().is_err());
}

#[test]
fn rejects_missing_name() {
    assert!("20260509_140000".parse::<Version>().is_err());
}

#[test]
fn ordering_by_timestamp() {
    let a: Version = "20260509_140000_a".parse().unwrap();
    let b: Version = "20260510_080000_b".parse().unwrap();
    assert!(a < b);
}

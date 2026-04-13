/// Verify that the prelude exposes all commonly used types.
use sntl::core::prelude::*;

#[test]
fn prelude_exposes_core_types() {
    // This test passes if it compiles — verifies the prelude re-exports work
    let _col = Column::new("t", "c");
    let _val = Value::from(42i64);
    let _q = SelectQuery::new("t");
    let _q = InsertQuery::new("t");
    let _q = UpdateQuery::new("t");
    let _q = DeleteQuery::new("t");
    let _q = QueryBuilder::select_from("t");
}

#[test]
fn driver_types_accessible() {
    // Verify driver types are accessible through sntl::driver
    let _ = sntl::driver::types::interval::PgInterval {
        months: 0,
        days: 0,
        microseconds: 0,
    };
    let _ = sntl::driver::types::geometric::PgPoint { x: 0.0, y: 0.0 };
    let _ = sntl::driver::types::network::PgMacAddr([0; 6]);
}

#[test]
fn reexports_driver_derives_and_traits() {
    // Verify re-export paths compile
    fn _assert_tosql<T: sntl::ToSql>() {}
    fn _assert_fromsql<T: sntl::FromSql>() {}
}

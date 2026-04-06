/// Verify that the prelude exposes all commonly used types.
use sntl_core::prelude::*;

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

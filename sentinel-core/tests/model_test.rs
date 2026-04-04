use sentinel_core::expr::Column;
use sentinel_core::model::{Model, ModelColumn};
use sentinel_core::types::Value;

/// A manually implemented Model for testing.
/// In Phase 2, `derive(Model)` will generate this.
struct User;

impl Model for User {
    const TABLE: &'static str = "users";
    const PRIMARY_KEY: &'static str = "id";

    fn columns() -> &'static [ModelColumn] {
        &USER_COLUMNS
    }
}

static USER_COLUMNS: [ModelColumn; 4] = [
    ModelColumn {
        name: "id",
        column_type: "uuid",
        nullable: false,
        has_default: true,
    },
    ModelColumn {
        name: "email",
        column_type: "text",
        nullable: false,
        has_default: false,
    },
    ModelColumn {
        name: "name",
        column_type: "text",
        nullable: true,
        has_default: false,
    },
    ModelColumn {
        name: "created_at",
        column_type: "timestamptz",
        nullable: false,
        has_default: true,
    },
];

// Column constants (derive(Model) will generate these)
impl User {
    const ID: Column = Column {
        table: std::borrow::Cow::Borrowed("users"),
        name: std::borrow::Cow::Borrowed("id"),
    };
    const EMAIL: Column = Column {
        table: std::borrow::Cow::Borrowed("users"),
        name: std::borrow::Cow::Borrowed("email"),
    };
    const NAME: Column = Column {
        table: std::borrow::Cow::Borrowed("users"),
        name: std::borrow::Cow::Borrowed("name"),
    };
    const CREATED_AT: Column = Column {
        table: std::borrow::Cow::Borrowed("users"),
        name: std::borrow::Cow::Borrowed("created_at"),
    };
}

#[test]
fn model_has_table_name() {
    assert_eq!(User::TABLE, "users");
}

#[test]
fn model_has_primary_key() {
    assert_eq!(User::PRIMARY_KEY, "id");
}

#[test]
fn model_columns_returns_metadata() {
    let cols = User::columns();
    assert_eq!(cols.len(), 4);
    assert_eq!(cols[0].name, "id");
    assert!(!cols[1].nullable);
    assert!(cols[2].nullable); // name is Option<String>
    assert!(cols[3].has_default); // created_at has default
}

#[test]
fn model_column_constants_build_expressions() {
    let expr = User::EMAIL.eq("alice@example.com");
    assert_eq!(expr.to_sql(1), "\"users\".\"email\" = $1");
}

#[test]
fn model_find_builds_select() {
    let q = User::find();
    let (sql, _) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
}

#[test]
fn model_find_by_id_builds_select() {
    let q = User::find_by_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".* FROM \"users\" WHERE \"id\" = $1"
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn model_delete_by_id() {
    let q = User::delete(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1");
    assert_eq!(binds.len(), 1);
}

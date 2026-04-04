use sentinel_core::expr::Column;
use sentinel_core::model::{Model, ModelColumn};
use sentinel_core::types::Value;
use sentinel_core::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,

    #[sentinel(unique)]
    pub email: String,

    pub name: Option<String>,

    #[sentinel(default = "now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[test]
fn model_trait_table_name() {
    assert_eq!(User::TABLE, "users");
}

#[test]
fn model_trait_primary_key() {
    assert_eq!(User::PRIMARY_KEY, "id");
}

#[test]
fn model_trait_columns() {
    let cols = User::columns();
    assert_eq!(cols.len(), 4);
    assert_eq!(cols[0].name, "id");
    assert_eq!(cols[0].column_type, "uuid");
    assert!(!cols[0].nullable);
    assert!(cols[0].has_default);

    assert_eq!(cols[1].name, "email");
    assert!(!cols[1].nullable);
    assert!(!cols[1].has_default);

    assert_eq!(cols[2].name, "name");
    assert!(cols[2].nullable);

    assert_eq!(cols[3].name, "created_at");
    assert!(cols[3].has_default);
}

#[test]
fn model_find() {
    let q = User::find();
    let (sql, _) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\"");
}

#[test]
fn model_find_by_id() {
    let q = User::find_by_id(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(sql, "SELECT \"users\".* FROM \"users\" WHERE \"id\" = $1");
    assert_eq!(binds.len(), 1);
}

#[test]
fn model_delete() {
    let q = User::delete(Value::from("abc-123"));
    let (sql, binds) = q.build();
    assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1");
    assert_eq!(binds.len(), 1);
}

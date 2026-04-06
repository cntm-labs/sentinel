use sntl_core::{Model, Partial};

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

#[derive(Partial)]
#[sentinel(model = "User")]
pub struct UserSummary {
    pub id: uuid::Uuid,
    pub name: Option<String>,
}

#[test]
fn partial_select_query() {
    let q = UserSummary::select_query();
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"name\" FROM \"users\""
    );
    assert!(binds.is_empty());
}

#[test]
fn partial_select_with_where() {
    let q = UserSummary::select_query().where_(User::NAME.is_not_null());
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"name\" FROM \"users\" WHERE \"users\".\"name\" IS NOT NULL"
    );
}

#[test]
fn partial_select_with_limit() {
    let q = UserSummary::select_query().limit(10);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "SELECT \"users\".\"id\", \"users\".\"name\" FROM \"users\" LIMIT 10"
    );
}

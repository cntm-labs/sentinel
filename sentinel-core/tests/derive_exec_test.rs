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

// Compile-time tests: verify async methods have correct signatures
#[allow(dead_code, unused_variables)]
async fn find_all_compiles(conn: &mut sentinel_driver::Connection) {
    let users: Vec<User> = User::find_all(conn).await.unwrap();
}

#[allow(dead_code, unused_variables)]
async fn find_one_compiles(conn: &mut sentinel_driver::Connection) {
    let user: User = User::find_one(conn, &uuid::Uuid::nil()).await.unwrap();
}

#[allow(dead_code, unused_variables)]
async fn find_optional_compiles(conn: &mut sentinel_driver::Connection) {
    let user: Option<User> = User::find_optional(conn, &uuid::Uuid::nil()).await.unwrap();
}

#[allow(dead_code, unused_variables)]
async fn create_exec_compiles(conn: &mut sentinel_driver::Connection) {
    let new = NewUser {
        email: "test@test.com".into(),
        name: None,
    };
    let user: User = User::create_exec(conn, new).await.unwrap();
}

#[allow(dead_code, unused_variables)]
async fn delete_by_id_compiles(conn: &mut sentinel_driver::Connection) {
    let affected: u64 = User::delete_by_id(conn, &uuid::Uuid::nil()).await.unwrap();
}

// Actual unit test (no connection needed)
#[test]
fn derive_model_generates_execution_methods() {
    // If this test file compiles, the methods exist with correct signatures.
    assert!(true);
}

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
fn new_user_has_correct_fields() {
    // NewUser should only have email and name (id and created_at have defaults)
    let new = NewUser {
        email: "alice@example.com".to_string(),
        name: Some("Alice".to_string()),
    };
    assert_eq!(new.email, "alice@example.com");
    assert_eq!(new.name, Some("Alice".to_string()));
}

#[test]
fn create_builds_insert_query() {
    let new = NewUser {
        email: "alice@example.com".to_string(),
        name: Some("Alice".to_string()),
    };
    let q = User::create(new);
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\", \"name\") VALUES ($1, $2) RETURNING *"
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0], Value::Text("alice@example.com".into()));
}

#[test]
fn create_with_none_optional() {
    let new = NewUser {
        email: "bob@example.com".to_string(),
        name: None,
    };
    let q = User::create(new);
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"users\" (\"email\", \"name\") VALUES ($1, $2) RETURNING *"
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[1], Value::Null);
}

#[test]
fn all_default_fields_model() {
    // A model where only the PK has a default
    #[derive(Model)]
    #[sentinel(table = "tags")]
    pub struct Tag {
        #[sentinel(primary_key, default = "gen_random_uuid()")]
        pub id: uuid::Uuid,

        pub label: String,
    }

    let new = NewTag {
        label: "rust".to_string(),
    };
    let q = Tag::create(new);
    let (sql, _) = q.build();
    assert_eq!(
        sql,
        "INSERT INTO \"tags\" (\"label\") VALUES ($1) RETURNING *"
    );
}

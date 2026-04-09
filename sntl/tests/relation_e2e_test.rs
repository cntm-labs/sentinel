use sntl::core::relation::*;
use sntl::{Model, sentinel};

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Model)]
#[sentinel(table = "posts")]
pub struct Post {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub title: String,
    pub published: bool,
}

#[sentinel(relations)]
impl User {
    pub fn posts() -> HasMany<Post> {
        HasMany::new("user_id")
    }
}

#[test]
fn relation_constant_exists() {
    let spec = User::POSTS;
    assert_eq!(spec.foreign_key(), "user_id");
    assert_eq!(spec.target_table(), "posts");
    assert_eq!(spec.kind(), RelationKind::HasMany);
}

#[test]
fn find_builds_select() {
    let (sql, _) = User::Find().Build();
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("users"));
}

#[test]
fn find_id_builds_where() {
    let (sql, binds) = User::FindId(42i32).Build();
    assert!(sql.contains("WHERE"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn relation_spec_filter_limit() {
    let spec = User::POSTS.Filter(Post::PUBLISHED.eq(true)).Limit(5);
    assert!(spec.has_filters());
    assert_eq!(spec.limit(), Some(5));
}

#[test]
fn batch_sql_generation() {
    let spec = User::POSTS;
    let (sql, binds) = spec.build_batch_sql(&[1i32.into(), 2i32.into()]);
    assert!(sql.contains("WHERE \"user_id\" IN ($1, $2)"));
    assert_eq!(binds.len(), 2);
}

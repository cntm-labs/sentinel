use sntl_core::Model;
use sntl_core::model::Model;

#[derive(Model)]
#[sentinel(table = "posts")]
pub struct Post {
    #[sentinel(primary_key)]
    pub id: i64,

    pub title: String,

    pub body: String,

    #[sentinel(default = "false")]
    pub published: bool,
}

#[test]
fn column_constant_id() {
    let expr = Post::ID.eq(42i64);
    assert_eq!(expr.to_sql(1), "\"posts\".\"id\" = $1");
}

#[test]
fn column_constant_title() {
    let expr = Post::TITLE.eq("Hello");
    assert_eq!(expr.to_sql(1), "\"posts\".\"title\" = $1");
}

#[test]
fn column_constant_published() {
    let expr = Post::PUBLISHED.eq(true);
    assert_eq!(expr.to_sql(1), "\"posts\".\"published\" = $1");
}

#[test]
fn column_constants_compose() {
    let expr = Post::TITLE.like("%rust%").and(Post::PUBLISHED.eq(true));
    let sql = expr.to_sql(1);
    assert!(sql.contains("$1"));
    assert!(sql.contains("$2"));
}

#[test]
fn column_constants_in_select() {
    let q = Post::find().where_(Post::PUBLISHED.eq(true));
    let (sql, binds) = q.build();
    assert_eq!(
        sql,
        "SELECT \"posts\".* FROM \"posts\" WHERE \"posts\".\"published\" = $1"
    );
    assert_eq!(binds.len(), 1);
}

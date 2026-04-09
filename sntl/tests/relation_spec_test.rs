use sntl::core::expr::Column;
use sntl::core::relation::{RelationKind, RelationSpec};

#[test]
fn relation_spec_basic() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany);
    assert_eq!(spec.name(), "posts");
    assert_eq!(spec.foreign_key(), "user_id");
    assert_eq!(spec.target_table(), "posts");
}

#[test]
fn relation_spec_with_filter() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany)
        .Filter(Column::new("posts", "published").eq(true))
        .Limit(5);
    assert_eq!(spec.limit(), Some(5));
    assert!(spec.has_filters());
}

#[test]
fn relation_spec_generates_where_in_sql() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany);
    let (sql, _) = spec.build_batch_sql(&[1i32.into(), 2i32.into()]);
    assert_eq!(
        sql,
        "SELECT \"posts\".* FROM \"posts\" WHERE \"user_id\" IN ($1, $2)"
    );
}

#[test]
fn relation_spec_with_filter_generates_sql() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany)
        .Filter(Column::new("posts", "published").eq(true))
        .OrderBy(Column::new("posts", "created_at").desc())
        .Limit(5);
    let (sql, _) = spec.build_batch_sql(&[1i32.into()]);
    assert!(sql.contains("WHERE \"user_id\" IN ($1)"));
    assert!(sql.contains("AND \"posts\".\"published\" = $2"));
    assert!(sql.contains("ORDER BY"));
    assert!(sql.contains("LIMIT 5"));
}

#[test]
fn relation_spec_const_construction() {
    const POSTS: RelationSpec =
        RelationSpec::new_const("posts", "user_id", "posts", RelationKind::HasMany);
    assert_eq!(POSTS.name(), "posts");
    assert_eq!(POSTS.foreign_key(), "user_id");
}

#[test]
fn relation_spec_new_const_at_runtime() {
    // Exercise new_const at runtime for coverage (const eval is invisible to llvm-cov)
    let spec = RelationSpec::new_const("comments", "post_id", "comments", RelationKind::HasMany);
    assert_eq!(spec.name(), "comments");
    assert_eq!(spec.foreign_key(), "post_id");
    assert_eq!(spec.target_table(), "comments");
    assert_eq!(spec.kind(), RelationKind::HasMany);
    assert_eq!(spec.limit(), None);
    assert!(!spec.has_filters());
}

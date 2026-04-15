use sntl::core::Column;
use sntl::core::query::IncludeQuery;
use sntl::core::relation::*;

struct User;
struct UserPosts;

impl sntl::core::Model for User {
    const TABLE: &'static str = "users";
    const PRIMARY_KEY: &'static str = "id";
    fn columns() -> &'static [sntl::core::ModelColumn] {
        &[]
    }
    fn from_row(_row: &sntl::core::Row) -> sntl::driver::Result<Self> {
        Ok(User)
    }
    fn primary_key_value(&self) -> sntl::core::Value {
        sntl::core::Value::from(0i32)
    }
}

// Transition: including posts flips position 0
impl<B> IncludeTransition<User, (Unloaded, B), UserPosts> for () {
    type Next = (Loaded, B);
}

#[test]
fn include_query_tracks_specs() {
    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany);
    let q: IncludeQuery<User, (Unloaded, Unloaded)> = IncludeQuery::from_table("users");
    let q2 = q.include_rel::<UserPosts>(spec);
    let (sql, _params) = q2.Build();
    assert!(sql.contains("users"));
    assert_eq!(q2.included_specs().len(), 1);
}

#[test]
fn include_query_chains_where() {
    let q: IncludeQuery<User, (Unloaded,)> = IncludeQuery::from_table("users");
    let q = q.Where(Column::new("users", "active").eq(true));
    let (sql, _) = q.Build();
    assert!(sql.contains("active"));
}

#[test]
fn include_query_chains_order_limit() {
    let q: IncludeQuery<User, (Unloaded,)> = IncludeQuery::from_table("users");
    let q = q.OrderBy(Column::new("users", "name").asc()).Limit(10);
    let (sql, _) = q.Build();
    assert!(sql.contains("ORDER BY"));
    assert!(sql.contains("LIMIT 10"));
}

// Manual ModelRelations impl (macro generates in production)
impl sntl::core::relation::ModelRelations for User {
    type BareState = (Unloaded, Unloaded);
}

#[test]
fn model_query_include_transitions_to_include_query() {
    use sntl::core::query::ModelQuery;

    let spec = RelationSpec::new("posts", "user_id", "posts", RelationKind::HasMany);
    let inc = RelationInclude::<User, UserPosts>::new(spec);

    let q: ModelQuery<User> = ModelQuery::from_table("users");
    let q2 = q.Include(inc);
    assert_eq!(q2.included_specs().len(), 1);
}

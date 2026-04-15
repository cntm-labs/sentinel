use sntl::core::relation::*;
use sntl::{Model, sentinel};

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "0")]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Model)]
#[sentinel(table = "posts")]
pub struct Post {
    #[sentinel(primary_key, default = "0")]
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: String,
}

#[sentinel(relations)]
impl User {
    pub fn posts() -> HasMany<Post> {
        HasMany::new("user_id")
    }
}

#[test]
fn macro_generates_typed_include_method() {
    // User::Posts() should return RelationInclude<User, UserPosts>
    let _inc = User::Posts();
}

#[test]
fn macro_generates_model_relations() {
    // ModelRelations should be implemented with BareState
    fn assert_model_relations<M: ModelRelations>() {}
    assert_model_relations::<User>();
}

#[test]
fn include_query_type_transitions() {
    // This must compile — proves type-state chain works
    let _q = User::Find().Include(User::Posts());
}

#[test]
fn find_id_include_compiles() {
    let _q = User::FindId(1).Include(User::Posts());
}

#[test]
fn type_aliases_exist() {
    // These compile = type aliases are generated correctly
    fn assert_bare(_: &UserBare) {}
    fn assert_with_posts(_: &UserWithPosts) {}
    fn assert_full(_: &UserFull) {}

    let _ = (
        assert_bare as fn(&_),
        assert_with_posts as fn(&_),
        assert_full as fn(&_),
    );
}

#[test]
fn accessor_on_loaded_state() {
    let mut store = RelationStore::new();
    store.insert_decoded(
        "posts",
        vec![Post {
            id: 1,
            user_id: 1,
            title: "Hello".into(),
            body: "World".into(),
        }],
    );
    let user_with_posts: UserWithPosts = WithRelations::new(
        User {
            id: 1,
            name: "Alice".into(),
            email: "a@b.com".into(),
        },
        store,
    );
    // Accessor trait auto-imported via `use sntl::core::relation::*`
    // ... actually the trait is defined locally, not in sntl::core::relation
    // The trait is UserRelAccessors which is in the test module scope
    let posts = user_with_posts.posts();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].title, "Hello");
}

#[test]
fn deref_to_model_fields() {
    let user: UserBare = WithRelations::new(
        User {
            id: 42,
            name: "Bob".into(),
            email: "b@c.com".into(),
        },
        RelationStore::new(),
    );
    assert_eq!(user.id, 42);
    assert_eq!(user.name, "Bob");
}

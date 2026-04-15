use sntl::core::relation::*;
use sntl::{Model, sentinel};

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Model)]
#[sentinel(table = "posts")]
pub struct Post {
    #[sentinel(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub published: bool,
}

#[sentinel(relations)]
impl User {
    pub fn posts() -> HasMany<Post> {
        HasMany::new("user_id")
    }
}

fn main() {
    // This must NOT compile — posts() requires Loaded state
    let user: UserBare = WithRelations::new(
        User { id: 1, name: "test".into(), email: "t@t.com".into() },
        RelationStore::new(),
    );
    let _ = user.posts();
}

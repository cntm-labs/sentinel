use sntl::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    pub id: i64,
    pub email: String,
}

fn main() {}

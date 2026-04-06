use sntl::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key)]
    pub id: i64,
    #[sentinel(primary_key)]
    pub uuid: String,
}

fn main() {}

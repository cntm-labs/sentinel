use sntl::Model;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primay_key)]
    pub id: i64,
}

fn main() {}

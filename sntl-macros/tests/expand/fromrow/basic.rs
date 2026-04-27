use sntl::FromRow;

#[derive(FromRow)]
pub struct Summary {
    pub id: i32,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

fn main() {
    fn assert_from_row<T: sntl::__macro_support::FromRow>() {}
    assert_from_row::<Summary>();
}

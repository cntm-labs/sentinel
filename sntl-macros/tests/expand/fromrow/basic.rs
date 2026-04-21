use sntl::FromRow;

#[derive(FromRow)]
pub struct Summary {
    pub id: uuid::Uuid,
    pub email: String,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn main() {
    fn assert_from_row<T: sntl::__macro_support::FromRow>() {}
    assert_from_row::<Summary>();
}

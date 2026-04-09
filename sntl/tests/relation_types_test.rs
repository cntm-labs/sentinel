use sntl::core::relation::{BelongsTo, HasMany, HasOne, RelationKind};

#[test]
fn has_many_stores_foreign_key() {
    let rel = HasMany::<String>::new("user_id");
    assert_eq!(rel.foreign_key(), "user_id");
    assert_eq!(rel.kind(), RelationKind::HasMany);
}

#[test]
fn has_one_stores_foreign_key() {
    let rel = HasOne::<String>::new("user_id");
    assert_eq!(rel.foreign_key(), "user_id");
    assert_eq!(rel.kind(), RelationKind::HasOne);
}

#[test]
fn belongs_to_stores_foreign_key() {
    let rel = BelongsTo::<String>::new("user_id");
    assert_eq!(rel.foreign_key(), "user_id");
    assert_eq!(rel.kind(), RelationKind::BelongsTo);
}

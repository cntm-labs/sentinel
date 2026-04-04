use sentinel_core::model::Model;
use sentinel_core::Model as DeriveModel;

// Test: table name inferred from struct name (UserProfile → "user_profiles")
#[derive(DeriveModel)]
pub struct UserProfile {
    #[sentinel(primary_key)]
    pub id: i64,
    pub display_name: String,
}

#[test]
fn inferred_table_name() {
    assert_eq!(UserProfile::TABLE, "user_profiles");
}

// Test: column rename
#[derive(DeriveModel)]
#[sentinel(table = "items")]
pub struct Item {
    #[sentinel(primary_key)]
    pub id: i64,

    #[sentinel(column = "item_name")]
    pub name: String,
}

#[test]
fn column_rename() {
    let expr = Item::NAME.eq("Widget");
    assert_eq!(expr.to_sql(1), "\"items\".\"item_name\" = $1");
}

#[test]
fn column_rename_in_metadata() {
    let cols = Item::columns();
    assert_eq!(cols[1].name, "item_name");
}

// Test: skip field
#[derive(DeriveModel)]
#[sentinel(table = "products")]
pub struct Product {
    #[sentinel(primary_key)]
    pub id: i64,

    pub sku: String,

    #[sentinel(skip)]
    pub computed_label: String,
}

#[test]
fn skip_field_not_in_columns() {
    let cols = Product::columns();
    assert_eq!(cols.len(), 2); // id + sku, not computed_label
}

#[test]
fn skip_field_no_column_constant() {
    // Product should have ID and SKU constants but NOT COMPUTED_LABEL
    let _ = Product::ID;
    let _ = Product::SKU;
    // Product::COMPUTED_LABEL should not exist — compile error if uncommented
}

#[test]
fn skip_field_not_in_new_struct() {
    // id has no default, so NewProduct has id + sku (not computed_label)
    let new = NewProduct {
        id: 1,
        sku: "ABC-123".to_string(),
    };
    assert_eq!(new.sku, "ABC-123");
}

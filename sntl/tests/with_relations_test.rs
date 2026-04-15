use sntl::core::relation::{Loaded, RelationLoaded, RelationStore, Unloaded, WithRelations};

// Minimal struct for testing — not a real Model
struct SimpleModel {
    id: i32,
    name: String,
}

#[test]
fn with_relations_deref_to_model() {
    let model = SimpleModel {
        id: 1,
        name: "test".into(),
    };
    let wr: WithRelations<SimpleModel, ()> = WithRelations::bare(model);
    // Deref — access model fields directly
    assert_eq!(wr.id, 1);
    assert_eq!(wr.name, "test");
}

#[test]
fn with_relations_into_inner() {
    let model = SimpleModel {
        id: 42,
        name: "inner".into(),
    };
    let wr = WithRelations::bare(model);
    let m = wr.into_inner();
    assert_eq!(m.id, 42);
}

#[test]
fn with_relations_new_with_state() {
    let model = SimpleModel {
        id: 1,
        name: "test".into(),
    };
    let store = RelationStore::new();
    let wr: WithRelations<SimpleModel, (Loaded, Unloaded)> = WithRelations::new(model, store);
    assert_eq!(wr.id, 1);
    assert!(wr.relations().is_empty());
}

#[test]
fn relation_store_insert_and_get() {
    let mut store = RelationStore::new();
    assert!(store.is_empty());
    store.insert_decoded("posts", vec![42i32, 43, 44]);
    assert!(!store.is_empty());
    let posts: &Vec<i32> = store.get("posts").expect("posts should exist");
    assert_eq!(posts, &vec![42, 43, 44]);
}

#[test]
fn relation_store_get_missing_returns_none() {
    let store = RelationStore::new();
    let result: Option<&Vec<i32>> = store.get("nope");
    assert!(result.is_none());
}

#[test]
fn relation_store_default() {
    let store = RelationStore::default();
    assert!(store.is_empty());
}

// Marker type for a test relation
struct TestPosts;

// Manual impl — in production, macro generates this
impl RelationLoaded<TestPosts> for WithRelations<SimpleModel, (Loaded,)> {
    type Output = Vec<i32>;
    fn get_relation(&self) -> &Vec<i32> {
        self.relations()
            .get::<Vec<i32>>("posts")
            .expect("posts loaded")
    }
}

#[test]
fn relation_loaded_trait_gates_access() {
    let mut store = RelationStore::new();
    store.insert_decoded("posts", vec![1i32, 2, 3]);
    let wr: WithRelations<SimpleModel, (Loaded,)> = WithRelations::new(
        SimpleModel {
            id: 1,
            name: "test".into(),
        },
        store,
    );
    let posts: &Vec<i32> = wr.get_relation();
    assert_eq!(posts, &vec![1, 2, 3]);
}

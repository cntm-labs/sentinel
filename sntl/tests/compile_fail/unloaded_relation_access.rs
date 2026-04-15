use sntl::core::relation::*;

struct FakeModel;
struct FakePosts;

// Only impl for Loaded state — Unloaded should fail
impl RelationLoaded<FakePosts> for WithRelations<FakeModel, (Loaded,)> {
    type Output = Vec<i32>;
    fn get_relation(&self) -> &Vec<i32> {
        todo!()
    }
}

fn main() {
    let wr: WithRelations<FakeModel, (Unloaded,)> = WithRelations::new(
        FakeModel,
        RelationStore::new(),
    );
    // This should fail — Unloaded state doesn't impl RelationLoaded
    let _ = RelationLoaded::<FakePosts>::get_relation(&wr);
}

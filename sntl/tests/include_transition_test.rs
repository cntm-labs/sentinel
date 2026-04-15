use sntl::core::relation::*;

struct User;
struct UserPosts;
struct UserProfile;

// Manual transition impls (macro generates these in production)
impl<B> IncludeTransition<User, (Unloaded, B), UserPosts> for () {
    type Next = (Loaded, B);
}
impl<A> IncludeTransition<User, (A, Unloaded), UserProfile> for () {
    type Next = (A, Loaded);
}

#[test]
fn include_transition_compiles() {
    // This test passes if it compiles — verifies the trait + impls work
    fn assert_transition<M, S, Rel, N>()
    where
        (): IncludeTransition<M, S, Rel, Next = N>,
    {
    }

    assert_transition::<User, (Unloaded, Unloaded), UserPosts, (Loaded, Unloaded)>();
    assert_transition::<User, (Unloaded, Unloaded), UserProfile, (Unloaded, Loaded)>();
    // Including posts when profile already loaded
    assert_transition::<User, (Unloaded, Loaded), UserPosts, (Loaded, Loaded)>();
}

#[test]
fn relation_include_holds_spec() {
    let inc: RelationInclude<User, UserPosts> = RelationInclude::new(RelationSpec::new(
        "posts",
        "user_id",
        "posts",
        RelationKind::HasMany,
    ));
    assert_eq!(inc.spec().name(), "posts");
}

#[test]
fn relation_include_into_spec() {
    let inc: RelationInclude<User, UserPosts> = RelationInclude::new(RelationSpec::new(
        "posts",
        "user_id",
        "posts",
        RelationKind::HasMany,
    ));
    let spec = inc.into_spec();
    assert_eq!(spec.name(), "posts");
    assert_eq!(spec.kind(), RelationKind::HasMany);
}

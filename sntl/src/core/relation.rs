//! Relation type descriptors for compile-time N+1 prevention.
//!
//! Defines `HasMany<T>`, `HasOne<T>`, `BelongsTo<T>` descriptors
//! and `Loaded`/`Unloaded` state markers for type-state relations.

use std::marker::PhantomData;

/// Marker: relation data has been loaded from the database.
pub struct Loaded;

/// Marker: relation data has NOT been loaded (default state).
pub struct Unloaded;

/// Relation cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    HasMany,
    HasOne,
    BelongsTo,
}

/// One-to-many relation descriptor.
pub struct HasMany<T> {
    fk: &'static str,
    _target: PhantomData<T>,
}

impl<T> HasMany<T> {
    pub const fn new(foreign_key: &'static str) -> Self {
        Self {
            fk: foreign_key,
            _target: PhantomData,
        }
    }
    pub fn foreign_key(&self) -> &'static str {
        self.fk
    }
    pub fn kind(&self) -> RelationKind {
        RelationKind::HasMany
    }
}

/// One-to-one relation descriptor.
pub struct HasOne<T> {
    fk: &'static str,
    _target: PhantomData<T>,
}

impl<T> HasOne<T> {
    pub const fn new(foreign_key: &'static str) -> Self {
        Self {
            fk: foreign_key,
            _target: PhantomData,
        }
    }
    pub fn foreign_key(&self) -> &'static str {
        self.fk
    }
    pub fn kind(&self) -> RelationKind {
        RelationKind::HasOne
    }
}

/// Inverse relation descriptor (many-to-one).
pub struct BelongsTo<T> {
    fk: &'static str,
    _target: PhantomData<T>,
}

impl<T> BelongsTo<T> {
    pub const fn new(foreign_key: &'static str) -> Self {
        Self {
            fk: foreign_key,
            _target: PhantomData,
        }
    }
    pub fn foreign_key(&self) -> &'static str {
        self.fk
    }
    pub fn kind(&self) -> RelationKind {
        RelationKind::BelongsTo
    }
}

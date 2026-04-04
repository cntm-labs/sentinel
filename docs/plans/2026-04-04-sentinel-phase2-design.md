# Sentinel ORM — Phase 2: Derive Macros Design

**Date:** 2026-04-04
**Status:** Approved
**Scope:** `derive(Model)` + `derive(Partial)` in sentinel-macros

---

## Goal

Replace manual `Model` trait implementations with derive macros that generate type-safe, zero-cost code from annotated Rust structs. Achieve Prisma-level DX without sacrificing Diesel-level performance.

## Research-Driven Design Decisions

Based on analysis of Diesel, sqlx, SeaORM, Prisma, and Cornucopia:

| Pain Point | Source | Sentinel Solution |
|-----------|--------|-------------------|
| Dual schema (schema.rs + model) | Diesel | Rust struct = single source of truth |
| 5 derives per struct | Diesel | 1 derive: `#[derive(Model)]` |
| Entity boilerplate 50-100 lines/table | SeaORM | `#[derive(Model)]` on plain struct |
| Separate insert struct | Diesel, sqlx | Auto-generate `NewUser` (skip default fields) |
| Impenetrable error messages | Diesel | darling + proc_macro_error + domain-specific msgs |
| No compile-time partial select | sqlx, SeaORM | `#[derive(Partial)]` with field validation |
| Forced naming (`Model` struct) | SeaORM | Name structs anything |
| ActiveValue wrapper verbose | SeaORM | Direct field access, zero wrappers |
| Live DB required at compile | sqlx | No DB needed |

---

## derive(Model)

### Attribute Syntax

```rust
use sentinel::prelude::*;

#[derive(Model)]
#[sentinel(table = "users")]
pub struct User {
    #[sentinel(primary_key, default = "gen_random_uuid()")]
    pub id: Uuid,

    #[sentinel(unique)]
    pub email: String,

    pub name: Option<String>,

    #[sentinel(default = "now()")]
    pub created_at: DateTime<Utc>,
}
```

**Attributes:**
- `#[sentinel(table = "...")]` — table name (optional; inferred as pluralized snake_case if omitted)
- `#[sentinel(primary_key)]` — marks PK field (required, exactly one)
- `#[sentinel(default = "...")]` — DB-generated field, skipped from NewModel struct
- `#[sentinel(unique)]` — metadata for migration system (stored in ModelColumn)
- `#[sentinel(column = "...")]` — rename column if field name differs
- `#[sentinel(skip)]` — exclude field from DB mapping entirely

### Generated Code

From the `User` struct above, `derive(Model)` generates:

```rust
// 1. Model trait implementation
#[automatically_derived]
impl Model for User {
    const TABLE: &'static str = "users";
    const PRIMARY_KEY: &'static str = "id";
    fn columns() -> &'static [ModelColumn] {
        &[
            ModelColumn { name: "id", column_type: "uuid", nullable: false, has_default: true },
            ModelColumn { name: "email", column_type: "text", nullable: false, has_default: false },
            ModelColumn { name: "name", column_type: "text", nullable: true, has_default: false },
            ModelColumn { name: "created_at", column_type: "timestamptz", nullable: false, has_default: true },
        ]
    }
}

// 2. Column constants (inherent, not trait — better IDE autocomplete)
impl User {
    pub const ID: Column = Column { table: Cow::Borrowed("users"), name: Cow::Borrowed("id") };
    pub const EMAIL: Column = Column { table: Cow::Borrowed("users"), name: Cow::Borrowed("email") };
    pub const NAME: Column = Column { table: Cow::Borrowed("users"), name: Cow::Borrowed("name") };
    pub const CREATED_AT: Column = Column { table: Cow::Borrowed("users"), name: Cow::Borrowed("created_at") };
}

// 3. NewUser struct (fields without `default`)
pub struct NewUser {
    pub email: String,
    pub name: Option<String>,
}

// 4. create() method
impl User {
    pub fn create(new: NewUser) -> InsertQuery {
        InsertQuery::new("users")
            .column("email", new.email)
            .column("name", new.name)
    }
}
```

### Type-to-Column Mapping

| Rust Type | column_type | PostgreSQL |
|-----------|-------------|------------|
| `String` | `"text"` | TEXT |
| `i32` | `"int4"` | INTEGER |
| `i64` | `"int8"` | BIGINT |
| `f64` | `"float8"` | DOUBLE PRECISION |
| `bool` | `"bool"` | BOOLEAN |
| `Uuid` | `"uuid"` | UUID |
| `DateTime<Utc>` | `"timestamptz"` | TIMESTAMPTZ |
| `Vec<u8>` | `"bytea"` | BYTEA |
| `Option<T>` | same as T | nullable |

---

## derive(Partial)

### Syntax

```rust
#[derive(Partial)]
#[sentinel(model = "User")]
pub struct UserSummary {
    pub id: Uuid,
    pub name: Option<String>,
}
```

### Generated Code

```rust
// Compile-time validation: every field must exist on User with matching type
// If validation passes, generate:

impl UserSummary {
    pub fn select_query() -> SelectQuery {
        SelectQuery::new("users").columns(vec!["id", "name"])
    }
}
```

### Compile-Time Validation

The macro validates at compile time:
1. Every field in the Partial struct exists on the referenced Model
2. Field types match exactly (including Option wrapping)
3. The referenced model name resolves to a type that derives Model

Validation is implemented via generated trait bounds that produce clear errors:

```rust
// Generated validation code (simplified)
const _: () = {
    // If User doesn't have field "id" of type Uuid, this fails to compile
    fn _check_id(_: <User as HasColumn<"id">>::Type) where <User as HasColumn<"id">>::Type: Same<Uuid> {}
};
```

---

## Error Messages

Investment in error quality is a primary differentiator.

### Examples

**Missing primary key:**
```
error: struct `User` has no `#[sentinel(primary_key)]` field
  --> src/models.rs:3:1
   |
3  | pub struct User {
   | ^^^^^^^^^^^^^^^
   |
   = help: add `#[sentinel(primary_key)]` to one field
```

**Typo in attribute:**
```
error: unknown attribute `primay_key`
  --> src/models.rs:5:15
   |
5  |     #[sentinel(primay_key)]
   |               ^^^^^^^^^^
   |
   = help: did you mean `primary_key`?
```

**Partial field not on model:**
```
error: field `nonexistent` in `UserSummary` does not exist on model `User`
  --> src/models.rs:4:5
   |
4  |     pub nonexistent: String,
   |     ^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: available columns on `User`: id, email, name, created_at
```

**Partial field type mismatch:**
```
error: `UserSummary::email` has type `i32` but `User::email` is `String`
  --> src/models.rs:12:5
   |
12 |     pub email: i32,
   |     ^^^^^^^^^^^^^^
```

---

## Implementation Stack

- **darling** — declarative attribute parsing with free typo detection
- **syn 2** + **quote** — parsing and code generation
- **proc-macro-error2** — rich diagnostics with help/note annotations
- **proc-macro2** — span tracking for precise error locations

### Architecture

```
Parse (darling) → IR (ModelIR/PartialIR) → Codegen (quote)
```

Each phase is independently testable.

---

## Performance

### Compile-time
- darling single-pass parsing
- Minimal generated code (trait impls call library functions)
- No DB connection at compile time
- Target: <200ms per 100 models

### Runtime (zero-cost)
- Column constants: `const` + `Cow::Borrowed` — zero allocation
- NewUser: plain struct, no wrappers — zero overhead
- `create()`: returns InsertQuery — same codepath as manual construction
- Partial `select_query()`: returns SelectQuery — same codepath as manual
- No runtime reflection, no Box<dyn>, no serde — static dispatch only

Generated code = exact same code a developer would write by hand.

---

## Out of Scope (Future Phases)

- Relations (`has_many`, `belongs_to`) — Phase 3
- Connection/exec (`.one(&db)`, `.all(&db)`) — Phase 4
- `#[reducer]` transactions — Phase 5
- Audit trail (`audit = true`) — later phase
- `sentinel db push` — later phase

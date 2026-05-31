# `#[sntl::reducer]` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `#[sntl::reducer]` proc-macro that wraps any async fn taking `&mut Connection` in BEGIN/COMMIT/ROLLBACK with isolation control, panic safety, and `ReducerBegin/Commit/Rollback` event emission.

**Architecture:** New module `sntl-macros/src/reducer/{mod,codegen}.rs` mirroring the existing `sntl-macros/src/test/` layout. Registered in `sntl-macros/src/lib.rs` as `#[proc_macro_attribute] fn reducer`. Re-exported from `sntl` as `sntl::reducer`. Codegen wraps the user fn body in `AssertUnwindSafe(async move { ... }).catch_unwind().await` to catch panics, then matches on the result to commit/rollback the connection and emit the appropriate observability event.

**Tech Stack:** Rust 1.85+, `syn 2.x`, `quote`, `proc-macro2`, `proc-macro-error2` (consistent with existing macros). `futures = "0.3"` becomes a direct dependency of `sntl` for `FutureExt::catch_unwind`. Runtime tests require live PostgreSQL via `SNTL_TEST_DATABASE_URL` (`postgres://sentinel:sentinel_test@localhost:5432/postgres`).

**Branch:** `feat/sntl-reducer` worktree at `.worktrees/feat-sntl-reducer/` (branched off main).

**Spec:** `docs/plans/2026-05-22-sntl-reducer-design.md`.

---

## File Structure

### Created

| Path | Responsibility |
|---|---|
| `sntl-macros/src/reducer/mod.rs` | Module entry; re-exports `expand` from codegen |
| `sntl-macros/src/reducer/codegen.rs` | Args parser + `expand(attr, item) -> TokenStream` |
| `sntl-macros/tests/reducer_expand.rs` | trybuild driver — `pass` + `compile_fail` fixtures |
| `sntl-macros/tests/expand/reducer/basic.rs` | Default form sanity (no isolation) |
| `sntl-macros/tests/expand/reducer/with_isolation.rs` | `isolation = "serializable"` form |
| `sntl-macros/tests/compile_fail/reducer_not_async.rs` (+`.stderr`) | sync fn rejection |
| `sntl-macros/tests/compile_fail/reducer_no_conn_arg.rs` (+`.stderr`) | wrong first-arg type rejection |
| `sntl-macros/tests/compile_fail/reducer_bad_isolation.rs` (+`.stderr`) | invalid isolation string rejection |
| `sntl/tests/reducer_test.rs` | 4 live-PG integration tests |
| `docs/reducer-guide.md` | User-facing guide (~500 words) |

### Modified

| Path | Reason |
|---|---|
| `sntl-macros/src/lib.rs` | Register `#[proc_macro_attribute] fn reducer` |
| `sntl-macros/src/test/codegen.rs` | (none — reducer is independent of test macro) |
| `sntl/Cargo.toml` | Add `futures = { version = "0.3", default-features = false, features = ["std"] }` |
| `sntl/src/lib.rs` | Re-export `pub use macros::reducer;`; add `FutureExt` to `__macro_support` |
| `sntl/src/lib.rs` (crate-level doc) | Extend "Day-one DX" with `#[sntl::reducer]` example + guide link |
| `README.md` | Append Features bullet |
| `docs/migration-from-sqlx.md` | New section "Replacing `sqlx::Transaction` with `#[sntl::reducer]`" |
| `docs/observability-guide.md` | Strike "Reducer events dormant" line from Limitations |
| `CLAUDE.md` | Update "Key Patterns" reducer entry from forward-looking to present-tense |
| `~/.claude/projects/.../memory/roadmap_sentinel.md` | Add v0.6 section noting `#[reducer]` shipped |

---

## Task 1: Setup worktree + foundation dep wiring

**Files:**
- Create: `.worktrees/feat-sntl-reducer/` (git worktree)
- Modify: `sntl/Cargo.toml`
- Modify: `sntl/src/lib.rs` (add `FutureExt` re-export in `__macro_support`)

- [ ] **Step 1: Create worktree off main**

```bash
git worktree add .worktrees/feat-sntl-reducer -b feat/sntl-reducer main
cd .worktrees/feat-sntl-reducer
```

All subsequent steps run inside the worktree.

- [ ] **Step 2: Add futures direct dep**

Open `sntl/Cargo.toml`. Find the `[dependencies]` block. Append:

```toml
futures = { version = "0.3", default-features = false, features = ["std"] }
```

- [ ] **Step 3: Re-export `FutureExt` in `__macro_support`**

Open `sntl/src/lib.rs`. Locate the `__macro_support` module (around the existing `pub mod __macro_support`). Add the re-export:

```rust
#[doc(hidden)]
pub mod __macro_support {
    pub use crate::core::query::macro_support::*;
    pub use ::futures::FutureExt;
}
```

- [ ] **Step 4: Verify compile**

```bash
cargo check --workspace
```

Expected: clean, no errors.

- [ ] **Step 5: Commit**

```bash
git add sntl/Cargo.toml sntl/src/lib.rs
git commit -m "chore(sntl): add futures direct dep + FutureExt in __macro_support"
```

---

## Task 2: Scaffold `reducer` proc-macro (no-op stub)

**Files:**
- Create: `sntl-macros/src/reducer/mod.rs`
- Create: `sntl-macros/src/reducer/codegen.rs`
- Modify: `sntl-macros/src/lib.rs`
- Modify: `sntl/src/lib.rs`

- [ ] **Step 1: Create `sntl-macros/src/reducer/mod.rs`**

```rust
mod codegen;

pub use codegen::expand;
```

- [ ] **Step 2: Create `sntl-macros/src/reducer/codegen.rs` with no-op stub**

```rust
use proc_macro2::TokenStream;

pub fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Stub: return the user function unchanged. Filled in by Task 4.
    item
}
```

- [ ] **Step 3: Register the macro in `sntl-macros/src/lib.rs`**

Open `sntl-macros/src/lib.rs`. Find the existing `mod test;` line and the corresponding `#[proc_macro_attribute] fn test` registration. Add the same shape for `reducer`:

```rust
mod reducer;

#[proc_macro_attribute]
#[proc_macro_error2::proc_macro_error]
pub fn reducer(attr: TokenStream, item: TokenStream) -> TokenStream {
    reducer::expand(attr.into(), item.into()).into()
}
```

Place the `mod reducer;` declaration alphabetised among the other `mod` lines, and the `pub fn reducer` after `pub fn test`.

- [ ] **Step 4: Re-export from sntl in `sntl/src/lib.rs`**

Find the existing `pub use macros::test;` line. Add immediately after it:

```rust
/// Attribute macro — `#[sntl::reducer]` wraps an async fn in BEGIN/COMMIT/ROLLBACK with auto-rollback on Err/panic.
pub use macros::reducer;
```

- [ ] **Step 5: Verify compile**

```bash
cargo check --workspace
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add sntl-macros/src/reducer sntl-macros/src/lib.rs sntl/src/lib.rs
git commit -m "feat(sntl-macros): scaffold #[sntl::reducer] (no-op stub)"
```

---

## Task 3: trybuild `pass` fixtures — basic + with_isolation

**Files:**
- Create: `sntl-macros/tests/reducer_expand.rs`
- Create: `sntl-macros/tests/expand/reducer/basic.rs`
- Create: `sntl-macros/tests/expand/reducer/with_isolation.rs`

- [ ] **Step 1: Create the trybuild driver**

`sntl-macros/tests/reducer_expand.rs`:

```rust
#[test]
fn reducer_expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/reducer/basic.rs");
    t.pass("tests/expand/reducer/with_isolation.rs");
    t.compile_fail("tests/compile_fail/reducer_not_async.rs");
    t.compile_fail("tests/compile_fail/reducer_no_conn_arg.rs");
    t.compile_fail("tests/compile_fail/reducer_bad_isolation.rs");
}
```

- [ ] **Step 2: Create `basic.rs`**

```rust
//! Default form (no isolation arg) compiles and produces an async fn.

use sntl::driver::Connection;

#[sntl::reducer]
async fn transfer(conn: &mut Connection, _from: i32, _to: i32, _amount: i64) -> sntl::Result<()> {
    let _ = conn;
    Ok(())
}

fn main() {
    // Compile-time only — fn is not called.
    let _ = transfer;
}
```

- [ ] **Step 3: Create `with_isolation.rs`**

```rust
//! `isolation = "serializable"` arg compiles.

use sntl::driver::Connection;

#[sntl::reducer(isolation = "serializable")]
async fn cas(conn: &mut Connection, _id: i32) -> sntl::Result<i64> {
    let _ = conn;
    Ok(0)
}

fn main() {
    let _ = cas;
}
```

- [ ] **Step 4: Run trybuild — expect basic to PASS (stub returns input unchanged) but with_isolation to PASS too (stub ignores attr)**

```bash
cargo test -p sntl-macros --test reducer_expand reducer_expand -- --ignored
# Note: trybuild tests have a known issue — running with default --include-ignored
# is rarely needed. Just:
cargo test -p sntl-macros --test reducer_expand
```

Expected: both `pass` fixtures succeed. (The `compile_fail` fixtures will FAIL this run because they don't exist yet — that's expected. Don't proceed to fix them in this task.)

- [ ] **Step 5: Don't commit yet**

The compile_fail fixtures are not yet present, so trybuild will report missing files. We commit after Task 4 when the implementation lands.

---

## Task 4: Implement core codegen (BEGIN/COMMIT/ROLLBACK + events, no panic safety yet)

**Files:**
- Modify: `sntl-macros/src/reducer/codegen.rs`

- [ ] **Step 1: Replace stub with parser + codegen**

Open `sntl-macros/src/reducer/codegen.rs` and replace its full contents with:

```rust
use proc_macro2::{Span, TokenStream};
use proc_macro_error2::abort;
use quote::quote;
use syn::{FnArg, ItemFn, Pat};

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        // Task 6 will parse `isolation = "..."` here. For now reject any attr.
        let s = attr.into_iter().next().map(|t| t.span()).unwrap_or_else(Span::call_site);
        abort!(s, "#[sntl::reducer] does not accept args yet (isolation = … lands in Task 6)");
    }

    let input_fn: ItemFn = match syn::parse2(item) {
        Ok(f) => f,
        Err(e) => abort!(e.span(), "{}", e),
    };

    if input_fn.sig.asyncness.is_none() {
        abort!(input_fn.sig.fn_token.span, "#[sntl::reducer] requires async fn");
    }

    // Find the first arg's binding name. We do NOT validate the type here —
    // Task 5 adds the &mut Connection type check.
    let conn_ident = match input_fn.sig.inputs.iter().next() {
        Some(FnArg::Typed(pat_type)) => match &*pat_type.pat {
            Pat::Ident(ident) => &ident.ident,
            _ => abort!(
                pat_type.pat.span(),
                "first arg of #[sntl::reducer] must be a simple identifier (e.g. `conn: &mut Connection`)"
            ),
        },
        _ => abort!(
            input_fn.sig.fn_token.span,
            "#[sntl::reducer] requires a first arg `conn: &mut driver::Connection`"
        ),
    };

    let fn_name_str = input_fn.sig.ident.to_string();
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let body = &input_fn.block;

    quote! {
        #vis #sig {
            use ::std::time::Instant;
            use ::sntl::driver::Event;

            let __name: &'static str = #fn_name_str;
            #conn_ident.instrumentation().on_event(&Event::ReducerBegin { name: __name });
            let __start = Instant::now();

            #conn_ident.begin().await?;

            // No move on the inner async block — body captures `conn` by reference,
            // and the outer scope still needs it for commit/rollback after .await.
            // Panic safety lands in Task 7 (AssertUnwindSafe + catch_unwind).
            match (async { #body }).await {
                Ok(__r) => {
                    #conn_ident.commit().await?;
                    #conn_ident.instrumentation().on_event(&Event::ReducerCommit {
                        name: __name,
                        duration: __start.elapsed(),
                    });
                    Ok(__r)
                }
                Err(__e) => {
                    let __err = format!("{}", __e);
                    #conn_ident.rollback().await.ok();
                    #conn_ident.instrumentation().on_event(&Event::ReducerRollback {
                        name: __name,
                        error: &__err,
                    });
                    Err(__e)
                }
            }
        }
    }
}
```

Why `async { #body }` (not `async move`): the body captures `conn` (the user's first arg) by reference. The outer scope needs `conn` back after the await for `commit()` / `rollback()`. A non-move async block reborrows; `async move` would consume the binding. Task 7 swaps this for an `AssertUnwindSafe(async move {...}).catch_unwind()` form that uses a raw-pointer reborrow to keep both sides happy.

- [ ] **Step 2: Run trybuild `pass` fixtures**

```bash
cargo test -p sntl-macros --test reducer_expand reducer_expand
```

Expected: `basic` PASS, `with_isolation` FAIL (because attr arg not yet parsed — `with_isolation.rs` uses `isolation = "serializable"` which Task 4 rejects with `abort!`). The `compile_fail/*` fixtures will fail because they don't exist yet — that's fine.

Update the trybuild driver to skip `with_isolation` temporarily — comment it out:

```rust
// t.pass("tests/expand/reducer/with_isolation.rs"); // Task 6
```

Re-run:

```bash
cargo test -p sntl-macros --test reducer_expand reducer_expand
```

Expected: only `basic` runs, passes. The three `compile_fail/*` files still don't exist; comment them out too:

```rust
// t.compile_fail("tests/compile_fail/reducer_not_async.rs");      // Task 8
// t.compile_fail("tests/compile_fail/reducer_no_conn_arg.rs");    // Task 8
// t.compile_fail("tests/compile_fail/reducer_bad_isolation.rs");  // Task 8
```

Re-run:

```bash
cargo test -p sntl-macros --test reducer_expand reducer_expand
```

Expected: only `basic` exercised, PASS.

- [ ] **Step 3: Commit**

```bash
git add sntl-macros/src/reducer/codegen.rs sntl-macros/tests/reducer_expand.rs sntl-macros/tests/expand/reducer/basic.rs sntl-macros/tests/expand/reducer/with_isolation.rs
git commit -m "feat(sntl-macros): core #[reducer] codegen (BEGIN/COMMIT/ROLLBACK + 3 events)"
```

---

## Task 5: Validate first-arg type is `&mut Connection`

**Files:**
- Modify: `sntl-macros/src/reducer/codegen.rs`

- [ ] **Step 1: Add type validation helper**

Open `sntl-macros/src/reducer/codegen.rs`. Replace the simple first-arg extraction (the block that finds `conn_ident`) with stricter validation:

```rust
// Locate first arg and validate type.
let conn_ident = match input_fn.sig.inputs.iter().next() {
    Some(FnArg::Typed(pat_type)) => {
        // Type must be `&mut <something>::Connection` or `&mut Connection`.
        let ok = matches!(&*pat_type.ty, Type::Reference(r) if r.mutability.is_some() && type_path_ends_with(&r.elem, "Connection"));
        if !ok {
            abort!(
                pat_type.ty.span(),
                "first arg of #[sntl::reducer] must be `&mut driver::Connection` (got `{}`)",
                quote!(#pat_type.ty).to_string()
            );
        }
        match &*pat_type.pat {
            Pat::Ident(ident) => &ident.ident,
            _ => abort!(pat_type.pat.span(), "first arg of #[sntl::reducer] must be a simple identifier"),
        }
    }
    _ => abort!(input_fn.sig.fn_token.span, "#[sntl::reducer] requires a first arg `conn: &mut driver::Connection`"),
};
```

Add the helper at module scope (above `pub fn expand`):

```rust
fn type_path_ends_with(ty: &Type, name: &str) -> bool {
    let Type::Path(tp) = ty else { return false };
    tp.path.segments.last().map(|s| s.ident == name).unwrap_or(false)
}
```

- [ ] **Step 2: Re-run trybuild `basic`**

```bash
cargo test -p sntl-macros --test reducer_expand reducer_expand
```

Expected: still PASS for `basic.rs`.

- [ ] **Step 3: Commit**

```bash
git add sntl-macros/src/reducer/codegen.rs
git commit -m "feat(sntl-macros): validate #[reducer] first arg is &mut Connection"
```

---

## Task 6: Isolation level attr arg parser

**Files:**
- Modify: `sntl-macros/src/reducer/codegen.rs`
- Modify: `sntl-macros/tests/reducer_expand.rs` (uncomment `with_isolation` line)

- [ ] **Step 1: Add Args struct + Parse impl**

Open `sntl-macros/src/reducer/codegen.rs`. Add at the top (below imports):

```rust
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Meta, Token};
use syn::punctuated::Punctuated;

/// Parsed attribute args: `isolation = "..."` (optional).
struct Args {
    isolation: Option<IsolationKind>,
}

#[derive(Clone, Copy)]
enum IsolationKind {
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut isolation = None;
        if input.is_empty() {
            return Ok(Self { isolation });
        }
        let metas: Punctuated<Meta, Token![,]> = Punctuated::parse_terminated(input)?;
        for m in metas {
            match &m {
                Meta::NameValue(nv) if nv.path.is_ident("isolation") => {
                    let lit_str = match &nv.value {
                        syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => s,
                        _ => return Err(syn::Error::new_spanned(&nv.value, "expected string literal")),
                    };
                    isolation = Some(parse_isolation(lit_str)?);
                }
                _ => return Err(syn::Error::new_spanned(&m, "unknown #[sntl::reducer] arg (expected `isolation = \"...\"`)")),
            }
        }
        Ok(Self { isolation })
    }
}

fn parse_isolation(s: &LitStr) -> syn::Result<IsolationKind> {
    match s.value().as_str() {
        "read_committed"  => Ok(IsolationKind::ReadCommitted),
        "repeatable_read" => Ok(IsolationKind::RepeatableRead),
        "serializable"    => Ok(IsolationKind::Serializable),
        _ => Err(syn::Error::new_spanned(
            s,
            "isolation must be one of: read_committed, repeatable_read, serializable",
        )),
    }
}
```

- [ ] **Step 2: Replace the `if !attr.is_empty()` abort with the Args parser**

In `pub fn expand`, replace the early `if !attr.is_empty() { abort!... }` block with:

```rust
let args: Args = match syn::parse2(attr) {
    Ok(a) => a,
    Err(e) => abort!(e.span(), "{}", e),
};
```

- [ ] **Step 3: Emit `begin_with(...)` when `isolation` provided**

Replace the existing `#conn_ident.begin().await?;` line with:

```rust
let begin_expr = match args.isolation {
    None => quote! { #conn_ident.begin().await?; },
    Some(IsolationKind::ReadCommitted)  => quote! {
        #conn_ident.begin_with(
            ::sntl::driver::TransactionConfig::new()
                .isolation(::sntl::driver::IsolationLevel::ReadCommitted)
        ).await?;
    },
    Some(IsolationKind::RepeatableRead) => quote! {
        #conn_ident.begin_with(
            ::sntl::driver::TransactionConfig::new()
                .isolation(::sntl::driver::IsolationLevel::RepeatableRead)
        ).await?;
    },
    Some(IsolationKind::Serializable)   => quote! {
        #conn_ident.begin_with(
            ::sntl::driver::TransactionConfig::new()
                .isolation(::sntl::driver::IsolationLevel::Serializable)
        ).await?;
    },
};
```

Then in the final `quote!` block, splice it in where the old `.begin().await?;` line was:

```rust
quote! {
    #vis #sig {
        use ::std::time::Instant;
        use ::sntl::driver::Event;

        let __name: &'static str = #fn_name_str;
        #conn_ident.instrumentation().on_event(&Event::ReducerBegin { name: __name });
        let __start = Instant::now();

        #begin_expr

        match (async { #body }).await {
            // ... unchanged from Task 4
        }
    }
}
```

- [ ] **Step 4: Uncomment `with_isolation` in the trybuild driver**

`sntl-macros/tests/reducer_expand.rs`:

```rust
#[test]
fn reducer_expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/reducer/basic.rs");
    t.pass("tests/expand/reducer/with_isolation.rs");
    // t.compile_fail("tests/compile_fail/reducer_not_async.rs");      // Task 8
    // t.compile_fail("tests/compile_fail/reducer_no_conn_arg.rs");    // Task 8
    // t.compile_fail("tests/compile_fail/reducer_bad_isolation.rs");  // Task 8
}
```

- [ ] **Step 5: Run trybuild**

```bash
cargo test -p sntl-macros --test reducer_expand reducer_expand
```

Expected: both `basic` and `with_isolation` PASS.

- [ ] **Step 6: Commit**

```bash
git add sntl-macros/src/reducer/codegen.rs sntl-macros/tests/reducer_expand.rs
git commit -m "feat(sntl-macros): #[reducer(isolation = ...)] arg + begin_with emission"
```

---

## Task 7: Add panic safety via `AssertUnwindSafe + catch_unwind`

**Files:**
- Modify: `sntl-macros/src/reducer/codegen.rs`

- [ ] **Step 1: Wrap body in `AssertUnwindSafe(async move { ... }).catch_unwind().await`**

Open `sntl-macros/src/reducer/codegen.rs`. Locate the `match (async { #body }).await { ... }` block from Task 4/6. Replace it with:

```rust
let __conn_ptr: *mut ::sntl::driver::Connection = #conn_ident;
let __result = ::sntl::__macro_support::FutureExt::catch_unwind(
    ::std::panic::AssertUnwindSafe(async move {
        // SAFETY: `__conn_ptr` was derived from `#conn_ident` whose outer
        // borrow outlives this scope, and we don't run the outer borrow
        // concurrently with this future. Re-borrowing here is sound.
        let #conn_ident = unsafe { &mut *__conn_ptr };
        #body
    })
).await;

match __result {
    Ok(Ok(__r)) => {
        #conn_ident.commit().await?;
        #conn_ident.instrumentation().on_event(&Event::ReducerCommit {
            name: __name,
            duration: __start.elapsed(),
        });
        Ok(__r)
    }
    Ok(Err(__e)) => {
        let __err = format!("{}", __e);
        #conn_ident.rollback().await.ok();
        #conn_ident.instrumentation().on_event(&Event::ReducerRollback {
            name: __name,
            error: &__err,
        });
        Err(__e)
    }
    Err(__panic_payload) => {
        #conn_ident.rollback().await.ok();
        #conn_ident.instrumentation().on_event(&Event::ReducerRollback {
            name: __name,
            error: "panic",
        });
        ::std::panic::resume_unwind(__panic_payload);
    }
}
```

- [ ] **Step 2: Run trybuild `pass` fixtures**

```bash
cargo test -p sntl-macros --test reducer_expand reducer_expand
```

Expected: both `basic` and `with_isolation` PASS (codegen change is internal — public surface unchanged).

- [ ] **Step 3: Commit**

```bash
git add sntl-macros/src/reducer/codegen.rs
git commit -m "feat(sntl-macros): #[reducer] panic safety via AssertUnwindSafe + catch_unwind"
```

---

## Task 8: Compile-fail fixtures + diagnostics

**Files:**
- Create: `sntl-macros/tests/compile_fail/reducer_not_async.rs`
- Create: `sntl-macros/tests/compile_fail/reducer_no_conn_arg.rs`
- Create: `sntl-macros/tests/compile_fail/reducer_bad_isolation.rs`
- Modify: `sntl-macros/tests/reducer_expand.rs`

- [ ] **Step 1: Create `reducer_not_async.rs`**

```rust
//! `#[sntl::reducer]` on a sync fn is rejected.

use sntl::driver::Connection;

#[sntl::reducer]
fn not_async(conn: &mut Connection) -> sntl::Result<()> {
    let _ = conn;
    Ok(())
}

fn main() {}
```

- [ ] **Step 2: Create `reducer_no_conn_arg.rs`**

```rust
//! First arg must be `&mut driver::Connection`.

#[sntl::reducer]
async fn missing_conn(other: i32) -> sntl::Result<()> {
    let _ = other;
    Ok(())
}

fn main() {}
```

- [ ] **Step 3: Create `reducer_bad_isolation.rs`**

```rust
//! Invalid isolation string is rejected at expand time.

use sntl::driver::Connection;

#[sntl::reducer(isolation = "snapshot")]
async fn bad(conn: &mut Connection) -> sntl::Result<()> {
    let _ = conn;
    Ok(())
}

fn main() {}
```

- [ ] **Step 4: Uncomment compile_fail lines in trybuild driver**

`sntl-macros/tests/reducer_expand.rs`:

```rust
#[test]
fn reducer_expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/reducer/basic.rs");
    t.pass("tests/expand/reducer/with_isolation.rs");
    t.compile_fail("tests/compile_fail/reducer_not_async.rs");
    t.compile_fail("tests/compile_fail/reducer_no_conn_arg.rs");
    t.compile_fail("tests/compile_fail/reducer_bad_isolation.rs");
}
```

- [ ] **Step 5: First run captures the .stderr snapshots**

```bash
TRYBUILD=overwrite cargo test -p sntl-macros --test reducer_expand reducer_expand
```

This writes:
- `sntl-macros/tests/compile_fail/reducer_not_async.stderr`
- `sntl-macros/tests/compile_fail/reducer_no_conn_arg.stderr`
- `sntl-macros/tests/compile_fail/reducer_bad_isolation.stderr`

- [ ] **Step 6: Sanity-check the .stderr files**

Each must contain the expected diagnostic substring:
- `reducer_not_async.stderr` → "#[sntl::reducer] requires async fn"
- `reducer_no_conn_arg.stderr` → "first arg of #[sntl::reducer] must be `&mut driver::Connection`"
- `reducer_bad_isolation.stderr` → "isolation must be one of: read_committed, repeatable_read, serializable"

If a snapshot does not contain the expected substring, the validation in codegen is missing — go back to Task 5 / Task 6 and add it.

- [ ] **Step 7: Re-run without TRYBUILD=overwrite to verify snapshots are stable**

```bash
cargo test -p sntl-macros --test reducer_expand reducer_expand
```

Expected: all 5 fixtures (2 pass + 3 compile_fail) succeed.

- [ ] **Step 8: Commit**

```bash
git add sntl-macros/tests/compile_fail/reducer_*.rs sntl-macros/tests/compile_fail/reducer_*.stderr sntl-macros/tests/reducer_expand.rs
git commit -m "test(sntl-macros): compile_fail fixtures for #[reducer] signature errors"
```

---

## Task 9: Live-PG integration tests (all 4 in one file)

**Files:**
- Create: `sntl/tests/reducer_test.rs`

This task creates one test file with all four scenarios from the design §7.2. Implementing them together keeps shared setup (Recorder, Pool helper) in one place.

- [ ] **Step 1: Verify the PG container is running**

```bash
podman ps --filter name=sntl-test-pg --format '{{.Names}} {{.Status}}'
```

If not running:

```bash
podman run -d --name sntl-test-pg \
  -e POSTGRES_USER=sentinel -e POSTGRES_PASSWORD=sentinel_test -e POSTGRES_DB=sentinel_test \
  -p 5432:5432 postgres:17
```

Then seed the integration schema:

```bash
podman exec sntl-test-pg psql -U sentinel -d sentinel_test -f - < tests/integration/setup.sql
```

- [ ] **Step 2: Create `sntl/tests/reducer_test.rs`**

```rust
//! Live-PG integration tests for `#[sntl::reducer]`.
//! Skips silently without DATABASE_URL.

use std::sync::{Arc, Mutex};

use sntl::driver::{Config, Connection, Event, Instrumentation};

#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        match ev {
            Event::ReducerBegin { name } => {
                self.0.lock().unwrap().push(format!("begin:{name}"));
            }
            Event::ReducerCommit { name, .. } => {
                self.0.lock().unwrap().push(format!("commit:{name}"));
            }
            Event::ReducerRollback { name, error } => {
                self.0.lock().unwrap().push(format!("rollback:{name}:{error}"));
            }
            _ => {}
        }
    }
}

async fn fresh_conn(rec: Arc<Recorder>) -> Option<Connection> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = Config::parse(&url).ok()?.with_instrumentation(rec);
    Connection::connect(cfg).await.ok()
}

async fn truncate(conn: &mut Connection) {
    conn.execute("TRUNCATE users RESTART IDENTITY CASCADE", &[])
        .await
        .ok();
}

// ────────── Scenario 1: success commits ──────────

#[sntl::reducer]
async fn insert_one(conn: &mut Connection, name: &str) -> sntl::Result<()> {
    sntl::query!(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        name, format!("{name}@example.com")
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

#[tokio::test]
async fn success_commits() {
    let rec = Arc::new(Recorder::default());
    let Some(mut conn) = fresh_conn(rec.clone()).await else { return };
    truncate(&mut conn).await;

    insert_one(&mut conn, "alice").await.unwrap();

    let n: i64 = sntl::query_unchecked!("SELECT COUNT(*)::int8 FROM users")
        .fetch_one::<(i64,)>(&mut conn).await.unwrap().0;
    assert_eq!(n, 1);

    let events = rec.0.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], "begin:insert_one");
    assert!(events[1].starts_with("commit:insert_one"));
}

// ────────── Scenario 2: error rollbacks ──────────

#[sntl::reducer]
async fn insert_then_fail(conn: &mut Connection, name: &str) -> sntl::Result<()> {
    sntl::query!(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        name, format!("{name}@example.com")
    )
    .execute(&mut *conn)
    .await?;
    Err(sntl::Error::Other("forced failure".into()))
}

#[tokio::test]
async fn error_rollbacks() {
    let rec = Arc::new(Recorder::default());
    let Some(mut conn) = fresh_conn(rec.clone()).await else { return };
    truncate(&mut conn).await;

    let res = insert_then_fail(&mut conn, "bob").await;
    assert!(res.is_err());

    let n: i64 = sntl::query_unchecked!("SELECT COUNT(*)::int8 FROM users")
        .fetch_one::<(i64,)>(&mut conn).await.unwrap().0;
    assert_eq!(n, 0, "rollback should erase the insert");

    let events = rec.0.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], "begin:insert_then_fail");
    assert!(events[1].starts_with("rollback:insert_then_fail:forced failure"));
}

// ────────── Scenario 3: panic rollbacks + resumes ──────────

#[sntl::reducer]
async fn insert_then_panic(conn: &mut Connection, name: &str) -> sntl::Result<()> {
    sntl::query!(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        name, format!("{name}@example.com")
    )
    .execute(&mut *conn)
    .await?;
    panic!("synthetic panic");
}

#[tokio::test]
async fn panic_rollbacks_and_resumes() {
    use std::panic::AssertUnwindSafe;
    use futures::FutureExt;

    let rec = Arc::new(Recorder::default());
    let Some(mut conn) = fresh_conn(rec.clone()).await else { return };
    truncate(&mut conn).await;

    let panicked = AssertUnwindSafe(insert_then_panic(&mut conn, "carol"))
        .catch_unwind()
        .await
        .is_err();
    assert!(panicked, "panic should propagate past the reducer wrapper");

    let n: i64 = sntl::query_unchecked!("SELECT COUNT(*)::int8 FROM users")
        .fetch_one::<(i64,)>(&mut conn).await.unwrap().0;
    assert_eq!(n, 0, "rollback should erase the insert even on panic");

    let events = rec.0.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], "begin:insert_then_panic");
    assert_eq!(events[1], "rollback:insert_then_panic:panic");
}

// ────────── Scenario 4: serializable isolation ──────────

#[sntl::reducer(isolation = "serializable")]
async fn cas_inc(conn: &mut Connection, id: i32) -> sntl::Result<i64> {
    let current: i64 = sntl::query_unchecked!("SELECT counter::int8 FROM kv WHERE id = $1")
        .fetch_one::<(i64,)>(&mut *conn).await?.0;
    sntl::query!("UPDATE kv SET counter = $1 WHERE id = $2", current + 1, id)
        .execute(&mut *conn).await?;
    Ok(current + 1)
}

#[tokio::test]
async fn serializable_isolation_applied() {
    let rec = Arc::new(Recorder::default());
    let Some(mut conn1) = fresh_conn(rec.clone()).await else { return };
    let Some(mut conn2) = fresh_conn(rec.clone()).await else { return };

    // Seed kv table fresh for this test.
    conn1.execute("DROP TABLE IF EXISTS kv", &[]).await.unwrap();
    conn1.execute("CREATE TABLE kv (id int PRIMARY KEY, counter int)", &[]).await.unwrap();
    conn1.execute("INSERT INTO kv (id, counter) VALUES (1, 0)", &[]).await.unwrap();

    // Race two CAS increments on the same row. Serializable isolation guarantees
    // one of them will see PG error code 40001 (serialization_failure).
    let (r1, r2) = tokio::join!(cas_inc(&mut conn1, 1), cas_inc(&mut conn2, 1));

    // Either both succeed (no race observed) or exactly one sees 40001.
    let ok_count = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    let err_count = 2 - ok_count;
    assert!(
        ok_count >= 1 && err_count <= 1,
        "exactly one reducer may fail with serialization_failure (got r1={:?}, r2={:?})",
        r1, r2
    );
}
```

- [ ] **Step 3: Run the test suite vs live PG**

```bash
DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test \
  cargo test -p sntl --test reducer_test -- --test-threads=1
```

Expected: 4 passed.

If `success_commits` fails because cache entry for the INSERT/COUNT query is missing, switch the `sntl::query!()` calls to `sntl::query_unchecked!()` in this file (the test verifies the macro layer, not query caching). The COUNT lines already use `query_unchecked!()`.

- [ ] **Step 4: Commit**

```bash
git add sntl/tests/reducer_test.rs
git commit -m "test(sntl): integration coverage for #[reducer] (commit/rollback/panic/isolation)"
```

---

## Task 10: User guide — `docs/reducer-guide.md`

**Files:**
- Create: `docs/reducer-guide.md`

- [ ] **Step 1: Write the guide**

Save the following as `docs/reducer-guide.md`:

```markdown
# `#[sntl::reducer]` Guide

A transactional reducer wraps an async fn in `BEGIN` / `COMMIT` / `ROLLBACK` with
automatic rollback on `Err`, automatic rollback on panic, and observability
events. It is the recommended way to write multi-query atomic operations in
Sentinel.

## 30-second quickstart

```rust
use sntl::driver::Connection;

#[sntl::reducer]
async fn transfer(conn: &mut Connection, from: i32, to: i32, amount: i64)
    -> sntl::Result<()>
{
    sntl::query!("UPDATE accounts SET balance = balance - $1 WHERE id = $2", amount, from)
        .execute(&mut *conn).await?;
    sntl::query!("UPDATE accounts SET balance = balance + $1 WHERE id = $2", amount, to)
        .execute(&mut *conn).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> sntl::Result<()> {
    let pool = /* ... */;
    let mut conn = pool.acquire().await?;
    transfer(&mut conn, 1, 2, 100).await?;
    Ok(())
}
```

## Three forms

**1. Default (driver default isolation = ReadCommitted):**

```rust
#[sntl::reducer]
async fn name(conn: &mut Connection, ...) -> Result<R, E> { ... }
```

**2. Explicit isolation:**

```rust
#[sntl::reducer(isolation = "serializable")]
async fn name(conn: &mut Connection, ...) -> Result<R, E> { ... }
```

Valid values: `"read_committed"`, `"repeatable_read"`, `"serializable"`.

**3. Panic-safe (automatic — no extra syntax):**

If the body panics, the transaction is rolled back, `ReducerRollback` fires
with `error: "panic"`, and the panic is re-raised. The connection is left
in a clean state.

## Observability

Every `#[sntl::reducer]` fn emits three events through the configured
`Instrumentation` impl:

| Event | When |
|---|---|
| `ReducerBegin { name }` | Right after `BEGIN`, before the body runs |
| `ReducerCommit { name, duration }` | After successful `COMMIT` |
| `ReducerRollback { name, error }` | After `ROLLBACK` (error returned or panic caught) |

Use `sntl::observability::install_default_tracing(pool)` to forward these to
any `tracing`-compatible backend. See [observability-guide.md](observability-guide.md).

## Limitations (v0.6)

- **First arg must be `&mut Connection`.** Pool callers must
  `let mut conn = pool.acquire().await?;` and pass `&mut conn`. The
  `&Pool` first-arg form is tracked for v0.7+.
- **Nested reducers are NOT supported.** Calling one `#[sntl::reducer]`
  fn from inside another opens a second `BEGIN` on a connection already
  in a transaction; PostgreSQL warns and the inner `COMMIT` becomes a
  no-op. Use savepoints manually (`SAVEPOINT` SQL) if you need partial
  rollback.
- **`#![forbid(unsafe_code)]` is incompatible.** The macro emits an
  unsafe pointer reborrow to support `catch_unwind`. If your crate
  forbids unsafe, use the explicit `Transaction::begin` form instead:

  ```rust
  let mut tx = sntl::Transaction::begin(&mut conn).await?;
  /* queries through tx.conn() */
  tx.commit().await?;
  ```
- **Rollback failures poison the connection.** If `conn.rollback()`
  itself fails after a `Err`/panic (e.g. the network dropped), the
  connection is left mid-transaction. The next user from the pool will
  see "current transaction is aborted" until a successful `ROLLBACK`.
  Pool poisoning on rollback failure is tracked for v0.7.
- **`E: Display` required.** The macro emits `format!("{e}")` to build
  the `error` field of `ReducerRollback`. `sntl::Error`, `anyhow::Error`,
  and most `thiserror`-derived enums work out of the box.
```

- [ ] **Step 2: Commit**

```bash
git add docs/reducer-guide.md
git commit -m "docs: #[sntl::reducer] user guide"
```

---

## Task 11: Update existing docs + CLAUDE.md + crate-level lib.rs doc

**Files:**
- Modify: `README.md`
- Modify: `sntl/src/lib.rs` (crate-level doc only)
- Modify: `docs/migration-from-sqlx.md`
- Modify: `docs/observability-guide.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: README.md Features bullet**

Open `README.md`. Find the Features section. After the existing "**Fixture-isolated test harness**" bullet, append:

```markdown
- **Transactional reducers** — `#[sntl::reducer]` wraps any async fn in `BEGIN`/`COMMIT`/`ROLLBACK` with isolation control, panic safety, and `ReducerBegin/Commit/Rollback` events (see [`docs/reducer-guide.md`](docs/reducer-guide.md))
```

- [ ] **Step 2: `sntl/src/lib.rs` crate-level doc**

In the existing crate-level doc (the `//!` block at the top), after the `#[sntl::test]` bullet, append:

```rust
//! - **`#[sntl::reducer]`.** Wrap any async fn in `BEGIN`/`COMMIT`/`ROLLBACK`
//!   with panic safety and `ReducerBegin/Commit/Rollback` events. See
//!   `docs/reducer-guide.md`.
```

In the same file, add to the Guides list:

```rust
//! - `docs/reducer-guide.md` — wrap async fns in atomic transactions with `#[sntl::reducer]`
```

- [ ] **Step 3: `docs/migration-from-sqlx.md` new section**

Append at the end (after the existing "What's different on purpose" section):

```markdown
## 7. Replacing `sqlx::Transaction` with `#[sntl::reducer]`

sqlx:

```rust
let mut tx = pool.begin().await?;
sqlx::query!("UPDATE accounts SET balance = balance - $1 WHERE id = $2", amount, from)
    .execute(&mut *tx).await?;
sqlx::query!("UPDATE accounts SET balance = balance + $1 WHERE id = $2", amount, to)
    .execute(&mut *tx).await?;
tx.commit().await?;
```

Sentinel:

```rust
#[sntl::reducer]
async fn transfer(conn: &mut Connection, from: i32, to: i32, amount: i64) -> sntl::Result<()> {
    sntl::query!("UPDATE accounts SET balance = balance - $1 WHERE id = $2", amount, from)
        .execute(&mut *conn).await?;
    sntl::query!("UPDATE accounts SET balance = balance + $1 WHERE id = $2", amount, to)
        .execute(&mut *conn).await?;
    Ok(())
}

let mut conn = pool.acquire().await?;
transfer(&mut conn, 1, 2, 100).await?;
```

The reducer macro emits the `BEGIN`/`COMMIT`/`ROLLBACK` boilerplate for you,
adds panic safety (automatic rollback if the body panics), and emits
observability events. See [`docs/reducer-guide.md`](reducer-guide.md).
```

- [ ] **Step 4: `docs/observability-guide.md` — strike the dormant line**

Find the Limitations table row that begins:

```markdown
| **`Reducer*` events dormant** | `ReducerBegin`, `ReducerCommit`, and `ReducerRollback` are declared in `Event` and handled by `SntlTracing`, but the `#[reducer]` macro does not yet exist in `sntl-macros`. The handler arms are ready; they will activate once the macro lands. |
```

Replace the cell with:

```markdown
| **`Reducer*` events** | Emitted automatically by `#[sntl::reducer]`-wrapped fns. See [`docs/reducer-guide.md`](reducer-guide.md). |
```

- [ ] **Step 5: CLAUDE.md Key Patterns**

In `CLAUDE.md` find the line:

```markdown
- **Reducer pattern** for transactions: `#[reducer]` = auto-commit/rollback
```

Replace with:

```markdown
- **Reducer pattern** for transactions: `#[sntl::reducer]` = auto-commit/rollback + panic safety + observability events (`sntl-macros/src/reducer/`, `docs/reducer-guide.md`)
```

- [ ] **Step 6: Verify compile (doctests for sntl/src/lib.rs crate doc)**

```bash
cargo doc -p sntl --no-deps 2>&1 | tail -10
```

Expected: clean. If a doc-link is dead, fix it.

- [ ] **Step 7: Commit**

```bash
git add README.md sntl/src/lib.rs docs/migration-from-sqlx.md docs/observability-guide.md CLAUDE.md
git commit -m "docs: announce #[sntl::reducer] across README + guides + CLAUDE.md"
```

---

## Task 12: Memory roadmap update

**Files:**
- Modify: `/home/mrbt/.claude/projects/-home-mrbt-Desktop-workspaces-orm-repositories-sentinel/memory/roadmap_sentinel.md`

- [ ] **Step 1: Append v0.6 section**

Open the roadmap memory. Find the `## v0.5 — day-one DX cycle (delivered 2026-05-19)` section. Insert a new section above it:

```markdown
## v0.6 — closing doc-vs-code gaps (in flight)
- ✅ **`#[sntl::reducer]` macro** (PR pending, 2026-05-22) — wraps async fn in
  BEGIN/COMMIT/ROLLBACK with isolation control, panic safety via
  AssertUnwindSafe + catch_unwind, and ReducerBegin/Commit/Rollback event
  emission. Closes the largest doc-vs-code gap identified by the /graphify
  audit. The Event arms were declared in v0.4 observability + handled by
  SntlTracing, but the macro itself had never been written.
  - **Out of scope (tracked for v0.7+):** auto-reorder lock by ID (LockBatch
    API), `&Pool` first-arg form, savepoints / nested reducers, pool
    poisoning on rollback failure.

```

- [ ] **Step 2: Commit (memory files are tracked in `~/.claude/projects/...`, not in the repo)**

The memory file lives outside the Sentinel repo — no `git add` needed.
The change is persistent automatically through the Claude memory system.

---

## Task 13: PR wrap-up

**Files:** None modified — verification + branch ship only.

- [ ] **Step 1: Workspace verification**

```bash
cd .worktrees/feat-sntl-reducer
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5432/sentinel_test \
  cargo test --workspace -- --test-threads=1
```

Expected: fmt clean, clippy clean, all tests pass.

- [ ] **Step 2: Push the branch**

```bash
git push -u origin feat/sntl-reducer
```

- [ ] **Step 3: Open the PR**

```bash
gh pr create --title "feat(sntl-macros): #[sntl::reducer] proc-macro (v0.6 phase 1)" --body "$(cat <<'EOF'
## Summary

Ships `#[sntl::reducer]` — an attribute proc-macro that wraps any async fn taking `&mut Connection` in `BEGIN`/`COMMIT`/`ROLLBACK` with isolation control, panic safety, and `ReducerBegin/Commit/Rollback` event emission.

\`\`\`rust
#[sntl::reducer]
async fn transfer(conn: &mut Connection, from: i32, to: i32, amount: i64) -> sntl::Result<()> {
    sntl::query!("UPDATE accounts SET balance = balance - $1 WHERE id = $2", amount, from)
        .execute(&mut *conn).await?;
    sntl::query!("UPDATE accounts SET balance = balance + $1 WHERE id = $2", amount, to)
        .execute(&mut *conn).await?;
    Ok(())
}
\`\`\`

Closes the largest doc-vs-code gap identified by the `/graphify` audit (2026-05-22): `#[reducer]` is listed in CLAUDE.md as a "Key Pattern" and the observability layer has dormant `ReducerBegin/Commit/Rollback` event arms, but the proc-macro itself had never been written.

## Scope

- Auto `BEGIN`/`COMMIT`/`ROLLBACK` + 3 events.
- `#[reducer(isolation = "...")]` attr arg (read_committed / repeatable_read / serializable).
- Panic safety via `AssertUnwindSafe` + `FutureExt::catch_unwind`.
- First arg required to be `&mut driver::Connection`.

## Out of scope (tracked for v0.7+)

- Auto-reorder lock by ID — fundamentally a runtime lock-manager (`LockBatch` API), not a macro feature.
- `&Pool` first-arg form (auto-acquire connection) — two-line workaround works today.
- Savepoints / nested reducers.
- Pool poisoning on rollback failure — driver-side concern.

## Test plan

- [x] `cargo test -p sntl-macros --test reducer_expand` — 2 pass + 3 compile_fail fixtures green.
- [x] `cargo test -p sntl --test reducer_test -- --test-threads=1` vs live PG — 4 scenarios pass (commit / rollback / panic / serializable).
- [x] Full workspace serial run — green.
- [x] \`cargo clippy --workspace --all-targets -- -D warnings\` — clean.
- [x] \`cargo fmt --all -- --check\` — clean.
- [ ] CI: PG 16 + PG 17, Coverage, Clippy, Format, Test, SonarQube.

## Docs

- New: \`docs/reducer-guide.md\` (~500 words).
- Updated: README.md Features bullet, \`sntl/src/lib.rs\` crate doc, \`docs/migration-from-sqlx.md\`, \`docs/observability-guide.md\` (struck "Reducer events dormant"), CLAUDE.md Key Patterns.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 4: Report PR URL to user**

The `gh pr create` output is the PR URL. After CI runs and goes green, the PR is ready to squash-merge.

---

## Self-Review

### Spec coverage

| Spec section | Implementing task(s) |
|---|---|
| §1 Goal — auto-tx + events + isolation + panic safety | Tasks 4, 6, 7 |
| §2 User-visible API (3 forms) | Tasks 3 (fixtures), 9 (integration) |
| §2 Signature constraints | Task 5 (type check), Task 8 (compile_fail diagnostics) |
| §3 Architecture & codegen | Tasks 2 (skeleton), 4 (core), 7 (panic) |
| §3.3 Unsafe pointer note | Task 7 codegen + Task 10 docs |
| §3.4 Why `&mut Connection` | Task 10 docs |
| §4 Isolation arg parser | Task 6 |
| §5 Panic safety dep + `__macro_support` re-export | Task 1 |
| §6 `E: Display` constraint | Task 4 (format! emission); Task 10 docs |
| §7.1 Macro-level tests | Tasks 3, 8 |
| §7.2 Runtime tests | Task 9 |
| §7.3 Coverage handling | (no-op — already excluded) |
| §8 Docs surface (6 places) | Tasks 10, 11, 12 |
| §9 Open items | Task 13 PR body lists deferred items |

### Placeholder scan

Searched the plan for `TBD`, `TODO`, `FIXME`, "implement later", "add appropriate", "similar to Task" — none remain. Every step contains the actual code or command an engineer needs.

### Type consistency

- `IsolationKind::{ReadCommitted, RepeatableRead, Serializable}` (Task 6) maps cleanly to driver `IsolationLevel::{ReadCommitted, RepeatableRead, Serializable}`. ✓
- Macro emits `::sntl::__macro_support::FutureExt` (Task 7). Task 1 Step 3 re-exports `FutureExt` under that exact path. ✓
- Codegen emits `#conn_ident.begin().await?` / `#conn_ident.commit().await?` / `#conn_ident.rollback().await.ok()` / `#conn_ident.instrumentation().on_event(...)`. All four exist on the driver's concrete `Connection` (verified during spec writing). ✓
- Event arms `ReducerBegin/Commit/Rollback` — payload field names match the driver's declaration (`name`, `duration`, `error`). ✓

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-05-22-sntl-reducer-impl.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?

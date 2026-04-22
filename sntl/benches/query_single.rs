//! Placeholder for end-to-end `query!` round-trip benches. Building this
//! out needs a warm Postgres fixture and a tokio runtime per group; that
//! work is tracked as a v0.3 follow-up so the bench harness compiles and
//! runs in CI today even without a database.

use criterion::{Criterion, criterion_group, criterion_main};

fn bench_placeholder(c: &mut Criterion) {
    c.bench_function("noop", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, bench_placeholder);
criterion_main!(benches);

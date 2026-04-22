//! Microbenches for the proc-macro-time pipeline. Right now the only
//! per-call hot path that lives in plain Rust is `hash_sql`; the rest
//! (lookup + resolve) does I/O and is benched separately.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_hash_short(c: &mut Criterion) {
    c.bench_function("hash_sql_short", |b| {
        b.iter(|| sntl_schema::normalize::hash_sql(black_box("SELECT id FROM users WHERE id = $1")))
    });
}

fn bench_hash_long(c: &mut Criterion) {
    let sql = "SELECT u.id, u.name, u.email, u.created_at, p.id, p.title, p.body \
               FROM users u LEFT JOIN posts p ON p.user_id = u.id \
               WHERE u.active = true AND p.published = true \
               ORDER BY p.created_at DESC LIMIT 50";
    c.bench_function("hash_sql_long", |b| {
        b.iter(|| sntl_schema::normalize::hash_sql(black_box(sql)))
    });
}

criterion_group!(benches, bench_hash_short, bench_hash_long);
criterion_main!(benches);

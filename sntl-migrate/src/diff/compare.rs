use sntl_schema::schema::{Column, Schema, Table};

/// All structural diffs between two schemas. Ordering is meaningful for
/// emit: dependencies first (CREATE TABLE before its columns get touched).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    AddTable(Table),
    DropTable {
        name: String,
    },
    AddColumn {
        table: String,
        column: Column,
    },
    DropColumn {
        table: String,
        column: String,
    },
    AlterColumnType {
        table: String,
        column: String,
        from: String,
        to: String,
    },
    AlterColumnNullable {
        table: String,
        column: String,
        to: bool,
    },
    AlterColumnDefault {
        table: String,
        column: String,
        to: Option<String>,
    },
    AddPrimaryKey {
        table: String,
        columns: Vec<String>,
    },
    DropPrimaryKey {
        table: String,
    },
    AddUnique {
        table: String,
        columns: Vec<String>,
    },
    DropUnique {
        table: String,
        columns: Vec<String>,
    },
}

/// Compute `target_state` - `current_state` in terms of executable Changes.
///
/// `cache` = the desired state (committed `.sentinel/schema.toml`).
/// `live`  = what the DB currently shows.
///
/// FK changes are **out of v0.3 scope** — `pull_schema` doesn't populate them.
pub fn compare(cache: &Schema, live: &Schema) -> Vec<Change> {
    let mut out = Vec::new();

    for t in &cache.tables {
        if live.find_table(&t.name).is_none() {
            out.push(Change::AddTable(t.clone()));
        }
    }
    for t in &live.tables {
        if cache.find_table(&t.name).is_none() {
            out.push(Change::DropTable {
                name: t.name.clone(),
            });
        }
    }
    for cache_t in &cache.tables {
        let Some(live_t) = live.find_table(&cache_t.name) else {
            continue;
        };
        diff_columns(cache_t, live_t, &mut out);
        diff_pk(cache_t, live_t, &mut out);
        diff_unique(cache_t, live_t, &mut out);
    }

    out
}

fn diff_columns(cache_t: &Table, live_t: &Table, out: &mut Vec<Change>) {
    for c in &cache_t.columns {
        if live_t.columns.iter().any(|lc| lc.name == c.name) {
            continue;
        }
        out.push(Change::AddColumn {
            table: cache_t.name.clone(),
            column: c.clone(),
        });
    }
    for c in &live_t.columns {
        if cache_t.columns.iter().any(|cc| cc.name == c.name) {
            continue;
        }
        out.push(Change::DropColumn {
            table: cache_t.name.clone(),
            column: c.name.clone(),
        });
    }
    for cc in &cache_t.columns {
        let Some(lc) = live_t.columns.iter().find(|lc| lc.name == cc.name) else {
            continue;
        };
        if cc.pg_type.pg_type != lc.pg_type.pg_type {
            out.push(Change::AlterColumnType {
                table: cache_t.name.clone(),
                column: cc.name.clone(),
                from: lc.pg_type.pg_type.clone(),
                to: cc.pg_type.pg_type.clone(),
            });
        }
        if cc.nullable != lc.nullable {
            out.push(Change::AlterColumnNullable {
                table: cache_t.name.clone(),
                column: cc.name.clone(),
                to: cc.nullable,
            });
        }
        if cc.default != lc.default {
            out.push(Change::AlterColumnDefault {
                table: cache_t.name.clone(),
                column: cc.name.clone(),
                to: cc.default.clone(),
            });
        }
    }
}

fn diff_pk(cache_t: &Table, live_t: &Table, out: &mut Vec<Change>) {
    let cache_pk: Vec<String> = cache_t
        .columns
        .iter()
        .filter(|c| c.primary_key)
        .map(|c| c.name.clone())
        .collect();
    let live_pk: Vec<String> = live_t
        .columns
        .iter()
        .filter(|c| c.primary_key)
        .map(|c| c.name.clone())
        .collect();
    match (cache_pk.is_empty(), live_pk.is_empty()) {
        (false, true) => out.push(Change::AddPrimaryKey {
            table: cache_t.name.clone(),
            columns: cache_pk,
        }),
        (true, false) => out.push(Change::DropPrimaryKey {
            table: cache_t.name.clone(),
        }),
        (false, false) if cache_pk != live_pk => {
            out.push(Change::DropPrimaryKey {
                table: cache_t.name.clone(),
            });
            out.push(Change::AddPrimaryKey {
                table: cache_t.name.clone(),
                columns: cache_pk,
            });
        }
        _ => {}
    }
}

fn diff_unique(cache_t: &Table, live_t: &Table, out: &mut Vec<Change>) {
    for cc in &cache_t.columns {
        let lc = live_t.columns.iter().find(|lc| lc.name == cc.name);
        match (cc.unique, lc.map(|lc| lc.unique)) {
            (true, Some(false)) | (true, None) => {
                out.push(Change::AddUnique {
                    table: cache_t.name.clone(),
                    columns: vec![cc.name.clone()],
                });
            }
            (false, Some(true)) => {
                out.push(Change::DropUnique {
                    table: cache_t.name.clone(),
                    columns: vec![cc.name.clone()],
                });
            }
            _ => {}
        }
    }
}

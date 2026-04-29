#![cfg(feature = "online")]

//! Live-PostgreSQL helpers shared by `sntl prepare` and tests.
//!
//! Two operations:
//! - [`pull_schema`]: query `information_schema` + `pg_catalog` to materialise a [`Schema`].
//! - [`prepare_query`]: PARSE a SQL statement to capture parameter OIDs and result columns.

use crate::cache::{CacheEntry, ColumnInfo, ParamInfo, QueryKind, SourceLocation};
use crate::error::{Error, Result};
use crate::normalize::{hash_sql, normalize_sql};
use crate::schema::{Column, PgTypeRef, Schema, Table};

pub async fn pull_schema(conn_str: &str) -> Result<Schema> {
    let config = sentinel_driver::Config::parse(conn_str)
        .map_err(|e| Error::Introspect(format!("invalid connection string: {e}")))?;
    let mut client = sentinel_driver::Connection::connect(config)
        .await
        .map_err(|e| Error::Introspect(format!("connect: {e}")))?;

    // Pull tables and columns. We avoid pg_type joins here because column.udt_name
    // is sometimes not unique against pg_type.typname; callers can refine later.
    let rows = client
        .query(
            "SELECT c.table_schema, c.table_name, c.column_name, c.is_nullable, c.column_default,
                    c.data_type,
                    coalesce((SELECT t.oid::int4 FROM pg_catalog.pg_type t WHERE t.typname = c.udt_name LIMIT 1), 0)::int4 AS oid,
                    (pk.constraint_name IS NOT NULL) AS is_pk,
                    (uq.constraint_name IS NOT NULL) AS is_unique
             FROM information_schema.columns c
             LEFT JOIN information_schema.key_column_usage pk
                ON pk.table_schema = c.table_schema AND pk.table_name = c.table_name
               AND pk.column_name = c.column_name AND pk.constraint_name LIKE '%_pkey'
             LEFT JOIN information_schema.key_column_usage uq
                ON uq.table_schema = c.table_schema AND uq.table_name = c.table_name
               AND uq.column_name = c.column_name AND uq.constraint_name LIKE '%_key'
             WHERE c.table_schema NOT IN ('pg_catalog', 'information_schema')
             ORDER BY c.table_schema, c.table_name, c.ordinal_position",
            &[],
        )
        .await
        .map_err(|e| Error::Introspect(format!("query schema: {e}")))?;

    let mut tables: Vec<Table> = vec![];
    for row in rows {
        let schema_name: String = row
            .try_get(0)
            .map_err(|e| Error::Introspect(format!("decode table_schema: {e}")))?;
        let table_name: String = row
            .try_get(1)
            .map_err(|e| Error::Introspect(format!("decode table_name: {e}")))?;
        let col_name: String = row
            .try_get(2)
            .map_err(|e| Error::Introspect(format!("decode column_name: {e}")))?;
        let is_nullable: String = row
            .try_get(3)
            .map_err(|e| Error::Introspect(format!("decode is_nullable: {e}")))?;
        let default: Option<String> = row
            .try_get(4)
            .map_err(|e| Error::Introspect(format!("decode default: {e}")))?;
        let data_type: String = row
            .try_get(5)
            .map_err(|e| Error::Introspect(format!("decode data_type: {e}")))?;
        let oid: i32 = row
            .try_get(6)
            .map_err(|e| Error::Introspect(format!("decode oid: {e}")))?;
        let is_pk: bool = row
            .try_get(7)
            .map_err(|e| Error::Introspect(format!("decode is_pk: {e}")))?;
        let is_unique: bool = row
            .try_get(8)
            .map_err(|e| Error::Introspect(format!("decode is_unique: {e}")))?;

        let existing_index = tables
            .iter()
            .position(|t| t.schema == schema_name && t.name == table_name);
        let table = match existing_index {
            Some(i) => &mut tables[i],
            None => {
                tables.push(Table {
                    name: table_name.clone(),
                    schema: schema_name.clone(),
                    columns: vec![],
                    foreign_keys: vec![],
                });
                tables.last_mut().unwrap()
            }
        };
        table.columns.push(Column {
            name: col_name,
            pg_type: PgTypeRef::simple(&data_type),
            oid: oid as u32,
            nullable: is_nullable == "YES",
            primary_key: is_pk,
            unique: is_unique,
            default,
        });
    }

    let postgres_version: String = {
        let ver_rows = client
            .query("SELECT version()", &[])
            .await
            .map_err(|e| Error::Introspect(format!("server version: {e}")))?;
        ver_rows
            .first()
            .and_then(|r| r.try_get::<String>(0).ok())
            .as_deref()
            .and_then(|s| s.split_whitespace().nth(1))
            .unwrap_or("unknown")
            .to_string()
    };

    Ok(Schema {
        version: 1,
        postgres_version,
        generated_at: Some(epoch_now_iso()),
        source: Some(redact(conn_str)),
        tables,
        enums: vec![],
        composites: vec![],
    })
}

fn epoch_now_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("epoch:{}", now.as_secs())
}

fn redact(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let after_scheme = scheme_end + 3;
            return format!("{}{}", &url[..after_scheme], &url[at..]);
        }
    }
    url.to_string()
}

pub async fn prepare_query(
    conn_str: &str,
    sql: &str,
    locations: Vec<SourceLocation>,
) -> Result<CacheEntry> {
    let config = sentinel_driver::Config::parse(conn_str)
        .map_err(|e| Error::Introspect(format!("invalid connection string: {e}")))?;
    let mut client = sentinel_driver::Connection::connect(config)
        .await
        .map_err(|e| Error::Introspect(format!("connect: {e}")))?;

    let stmt = client
        .prepare(sql)
        .await
        .map_err(|e| Error::Introspect(format!("prepare: {e}")))?;

    let params: Vec<ParamInfo> = stmt
        .param_types()
        .iter()
        .enumerate()
        .map(|(i, oid)| ParamInfo {
            index: (i + 1) as u32,
            // sentinel-driver only exposes OID at prepare time; the human-readable
            // pg_type name is filled in later by the offline resolver against the
            // schema snapshot. Leaving it empty here keeps the boundary clean.
            pg_type: String::new(),
            oid: u32::from(*oid),
        })
        .collect();

    let columns: Vec<ColumnInfo> = stmt
        .columns()
        .map(<[_]>::to_vec)
        .unwrap_or_default()
        .into_iter()
        .map(|c| ColumnInfo {
            name: c.name,
            pg_type: String::new(),
            oid: c.type_oid,
            nullable: true, // refined by the offline analyzer; the server can't tell us at prepare time
            origin: None,
            element_type: None, // populated in Task 10's batched pg_type lookup
        })
        .collect();

    let normalized = normalize_sql(sql);
    let hash = hash_sql(sql);
    let upper = normalized.trim_start().to_ascii_uppercase();
    let kind = if upper.starts_with("SELECT") {
        QueryKind::Select
    } else if upper.starts_with("INSERT") {
        QueryKind::Insert
    } else if upper.starts_with("UPDATE") {
        QueryKind::Update
    } else if upper.starts_with("DELETE") {
        QueryKind::Delete
    } else {
        QueryKind::Other
    };
    let has_returning = upper.contains(" RETURNING ");

    Ok(CacheEntry {
        version: 1,
        sql_hash: hash,
        sql_normalized: normalized,
        source_locations: locations,
        params,
        columns,
        query_kind: kind,
        has_returning,
    })
}

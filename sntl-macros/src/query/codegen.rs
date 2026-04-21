//! Token-stream emitters shared by every macro variant.

use proc_macro2::TokenStream;
use quote::quote;
use sntl_schema::cache::{ColumnInfo, ParamInfo};
use syn::Expr;

pub struct CodegenInput<'a> {
    pub sql: &'a str,
    pub params: &'a [ParamInfo],
    pub param_exprs: &'a [Expr],
}

/// Build the `TypedQueryHandle::new(sql, &[oids])` expression.
pub fn build_handle(input: &CodegenInput) -> TokenStream {
    let sql = input.sql;
    let oids = input.params.iter().map(|p| p.oid);
    quote! {
        ::sntl::__macro_support::TypedQueryHandle::new(
            #sql,
            &[ #( ::sntl::Oid::from(#oids) ),* ],
        )
    }
}

/// Borrow each user-supplied expression as `&(dyn driver::ToSql + Sync)`.
pub fn build_params(input: &CodegenInput) -> TokenStream {
    let exprs = input.param_exprs;
    quote! {
        &[ #( &(#exprs) as &(dyn ::sntl::driver::ToSql + ::std::marker::Sync) ),* ]
    }
}

pub fn rust_type_for_column(c: &ColumnInfo) -> TokenStream {
    let base = rust_type_for_pg_oid(c.oid, &c.pg_type);
    if c.nullable {
        quote! { ::std::option::Option<#base> }
    } else {
        base
    }
}

/// Map a PostgreSQL type name (or, when absent, an OID) to the Rust type used
/// by the macro. The OID path matters because `sntl prepare` populates only
/// OIDs at PARSE time; the human-readable name is filled in later or empty.
pub fn rust_type_for_pg_oid(oid: u32, pg_type: &str) -> TokenStream {
    if !pg_type.is_empty() {
        return rust_type_for_pg(pg_type);
    }
    rust_type_for_oid(oid)
}

pub fn rust_type_for_oid(oid: u32) -> TokenStream {
    match oid {
        16 => quote! { bool },
        17 => quote! { ::std::vec::Vec<u8> },
        20 => quote! { i64 },
        21 => quote! { i16 },
        23 => quote! { i32 },
        25 | 1043 => quote! { ::std::string::String },
        700 => quote! { f32 },
        701 => quote! { f64 },
        1082 => quote! { ::chrono::NaiveDate },
        1083 => quote! { ::chrono::NaiveTime },
        1114 => quote! { ::chrono::NaiveDateTime },
        1184 => quote! { ::chrono::DateTime<::chrono::Utc> },
        2950 => quote! { ::uuid::Uuid },
        114 | 3802 => quote! { ::serde_json::Value },
        1700 => quote! { ::rust_decimal::Decimal },
        other => {
            let msg = format!(
                "unsupported PostgreSQL OID {other} — use query_as! with an explicit target struct, or extend rust_type_for_oid in sntl-macros::query::codegen"
            );
            quote! { compile_error!(#msg) }
        }
    }
}

pub fn rust_type_for_pg(pg_type: &str) -> TokenStream {
    match pg_type {
        "bool" | "boolean" => quote! { bool },
        "int2" | "smallint" => quote! { i16 },
        "int4" | "integer" => quote! { i32 },
        "int8" | "bigint" => quote! { i64 },
        "float4" | "real" => quote! { f32 },
        "float8" | "double precision" => quote! { f64 },
        "text" | "varchar" | "character varying" | "bpchar" | "char" => {
            quote! { ::std::string::String }
        }
        "bytea" => quote! { ::std::vec::Vec<u8> },
        "uuid" => quote! { ::uuid::Uuid },
        "timestamptz" | "timestamp with time zone" => {
            quote! { ::chrono::DateTime<::chrono::Utc> }
        }
        "timestamp" | "timestamp without time zone" => quote! { ::chrono::NaiveDateTime },
        "date" => quote! { ::chrono::NaiveDate },
        "time" | "time without time zone" => quote! { ::chrono::NaiveTime },
        "json" | "jsonb" => quote! { ::serde_json::Value },
        "numeric" | "decimal" => quote! { ::rust_decimal::Decimal },
        other => {
            let msg = format!(
                "unsupported PG type `{other}` — use query_as! with an explicit target struct, or add mapping in sntl-macros/src/query/codegen.rs"
            );
            quote! { compile_error!(#msg) }
        }
    }
}

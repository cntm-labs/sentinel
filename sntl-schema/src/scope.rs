use crate::error::{Error, Result};
use sqlparser::ast::{Join, JoinOperator, ObjectName, Query, SetExpr, TableFactor, TableWithJoins};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinKind {
    Base,
    Inner,
    LeftForcedNullable,
    RightForcedNullable,
    FullForcedNullable,
    Cross,
}

#[derive(Debug, Clone)]
pub struct TableRef {
    pub alias: String,
    pub table_name: String,
    pub schema: Option<String>,
    pub join_kind: JoinKind,
}

#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub tables: Vec<TableRef>,
}

impl Scope {
    pub fn resolve_alias(&self, name: &str) -> Option<&TableRef> {
        self.tables.iter().find(|t| t.alias == name)
    }
}

pub fn build_scope(query: &Query) -> Result<Scope> {
    let body = match &*query.body {
        SetExpr::Select(s) => s,
        _ => return Err(Error::SqlParse("scope only supports plain SELECT".into())),
    };
    let mut scope = Scope::default();
    for twj in &body.from {
        walk_twj(twj, &mut scope)?;
    }
    Ok(scope)
}

fn walk_twj(twj: &TableWithJoins, scope: &mut Scope) -> Result<()> {
    push_factor(&twj.relation, JoinKind::Base, scope)?;
    for j in &twj.joins {
        push_join(j, scope)?;
    }
    Ok(())
}

fn push_factor(factor: &TableFactor, kind: JoinKind, scope: &mut Scope) -> Result<()> {
    match factor {
        TableFactor::Table { name, alias, .. } => {
            let (schema, table) = split_name(name);
            let alias_name = alias
                .as_ref()
                .map(|a| a.name.value.clone())
                .unwrap_or_else(|| table.clone());
            scope.tables.push(TableRef {
                alias: alias_name,
                table_name: table,
                schema,
                join_kind: kind,
            });
            Ok(())
        }
        TableFactor::Derived { .. }
        | TableFactor::NestedJoin { .. }
        | TableFactor::TableFunction { .. }
        | TableFactor::UNNEST { .. } => Err(Error::SqlParse(
            "scope does not yet support derived tables, nested joins, or functions — use override or query_unchecked!".into(),
        )),
        _ => Err(Error::SqlParse("unsupported FROM factor".into())),
    }
}

fn push_join(j: &Join, scope: &mut Scope) -> Result<()> {
    let kind = match &j.join_operator {
        JoinOperator::Inner(_) | JoinOperator::Join(_) => JoinKind::Inner,
        JoinOperator::LeftOuter(_)
        | JoinOperator::Left(_)
        | JoinOperator::LeftSemi(_)
        | JoinOperator::LeftAnti(_) => JoinKind::LeftForcedNullable,
        JoinOperator::RightOuter(_)
        | JoinOperator::Right(_)
        | JoinOperator::RightSemi(_)
        | JoinOperator::RightAnti(_) => JoinKind::RightForcedNullable,
        JoinOperator::FullOuter(_) => JoinKind::FullForcedNullable,
        JoinOperator::CrossJoin | JoinOperator::CrossApply | JoinOperator::OuterApply => {
            JoinKind::Cross
        }
        _ => JoinKind::Inner,
    };
    push_factor(&j.relation, kind, scope)
}

fn split_name(name: &ObjectName) -> (Option<String>, String) {
    let parts: Vec<String> = name
        .0
        .iter()
        .filter_map(|p| p.as_ident().map(|i| i.value.clone()))
        .collect();
    match parts.len() {
        1 => (None, parts[0].clone()),
        2 => (Some(parts[0].clone()), parts[1].clone()),
        _ => (None, parts.last().cloned().unwrap_or_default()),
    }
}

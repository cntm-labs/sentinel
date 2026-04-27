use crate::error::{Error, Result};
use sqlparser::ast::{Query, Statement};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

pub enum ParsedStatement {
    Select(Box<Query>),
    Insert { body: Box<Statement> },
    Update { body: Box<Statement> },
    Delete { body: Box<Statement> },
    Other { body: Box<Statement> },
}

impl ParsedStatement {
    pub fn kind(&self) -> crate::cache::QueryKind {
        use crate::cache::QueryKind::*;
        match self {
            ParsedStatement::Select(_) => Select,
            ParsedStatement::Insert { .. } => Insert,
            ParsedStatement::Update { .. } => Update,
            ParsedStatement::Delete { .. } => Delete,
            ParsedStatement::Other { .. } => Other,
        }
    }
}

pub fn parse_statement(sql: &str) -> Result<ParsedStatement> {
    let dialect = PostgreSqlDialect {};
    let mut stmts =
        Parser::parse_sql(&dialect, sql).map_err(|e| Error::SqlParse(format!("{e}")))?;
    if stmts.is_empty() {
        return Err(Error::SqlParse("no statement".into()));
    }
    if stmts.len() > 1 {
        return Err(Error::SqlParse("expected exactly one statement".into()));
    }
    let stmt = stmts.remove(0);
    Ok(match stmt {
        Statement::Query(q) => ParsedStatement::Select(q),
        s @ Statement::Insert(_) => ParsedStatement::Insert { body: Box::new(s) },
        s @ Statement::Update { .. } => ParsedStatement::Update { body: Box::new(s) },
        s @ Statement::Delete(_) => ParsedStatement::Delete { body: Box::new(s) },
        s => ParsedStatement::Other { body: Box::new(s) },
    })
}

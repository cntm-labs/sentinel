use std::borrow::Cow;

use crate::types::Value;

/// A reference to a table column, used to build type-safe expressions.
#[derive(Debug, Clone)]
pub struct Column {
    pub table: Cow<'static, str>,
    pub name: Cow<'static, str>,
}

impl Column {
    pub fn new(table: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            table: Cow::Owned(table.into()),
            name: Cow::Owned(name.into()),
        }
    }

    /// Fully qualified and quoted column reference: `"table"."column"`
    pub fn qualified(&self) -> String {
        format!("\"{}\".\"{}\"", self.table, self.name)
    }

    pub fn eq(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "=",
            value: val.into(),
        }
    }

    pub fn ne(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "!=",
            value: val.into(),
        }
    }

    pub fn gt(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: ">",
            value: val.into(),
        }
    }

    pub fn lt(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "<",
            value: val.into(),
        }
    }

    pub fn gte(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: ">=",
            value: val.into(),
        }
    }

    pub fn lte(&self, val: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "<=",
            value: val.into(),
        }
    }

    pub fn like(&self, pattern: impl Into<Value>) -> Expr {
        Expr::Compare {
            column: self.qualified(),
            op: "LIKE",
            value: pattern.into(),
        }
    }

    pub fn is_null(&self) -> Expr {
        Expr::IsNull {
            column: self.qualified(),
            negated: false,
        }
    }

    pub fn is_not_null(&self) -> Expr {
        Expr::IsNull {
            column: self.qualified(),
            negated: true,
        }
    }

    pub fn in_list(&self, values: Vec<Value>) -> Expr {
        Expr::InList {
            column: self.qualified(),
            values,
        }
    }

    pub fn desc(&self) -> OrderExpr {
        OrderExpr {
            column: self.qualified(),
            direction: "DESC",
        }
    }

    pub fn asc(&self) -> OrderExpr {
        OrderExpr {
            column: self.qualified(),
            direction: "ASC",
        }
    }
}

/// An ordering expression for ORDER BY clauses.
#[derive(Debug, Clone)]
pub struct OrderExpr {
    column: String,
    direction: &'static str,
}

impl OrderExpr {
    pub fn to_sql_bare(&self) -> String {
        format!("{} {}", self.column, self.direction)
    }
}

/// A filter expression that generates parameterized SQL.
///
/// Bind parameter indices start from a given offset and increment sequentially.
/// This ensures composed expressions produce correct `$1, $2, ...` placeholders.
#[derive(Debug, Clone)]
pub enum Expr {
    Compare {
        column: String,
        op: &'static str,
        value: Value,
    },
    IsNull {
        column: String,
        negated: bool,
    },
    InList {
        column: String,
        values: Vec<Value>,
    },
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Generate SQL with bind parameters starting at the given index.
    pub fn to_sql(&self, start: usize) -> String {
        match self {
            Expr::Compare { column, op, .. } => {
                format!("{column} {op} ${start}")
            }
            Expr::IsNull { column, negated } => {
                if *negated {
                    format!("{column} IS NOT NULL")
                } else {
                    format!("{column} IS NULL")
                }
            }
            Expr::InList { column, values } => {
                let placeholders: Vec<String> = (0..values.len())
                    .map(|i| format!("${}", start + i))
                    .collect();
                format!("{column} IN ({})", placeholders.join(", "))
            }
            Expr::And(left, right) => {
                let left_sql = left.to_sql(start);
                let left_count = left.bind_count();
                let right_sql = right.to_sql(start + left_count);
                format!("({left_sql} AND {right_sql})")
            }
            Expr::Or(left, right) => {
                let left_sql = left.to_sql(start);
                let left_count = left.bind_count();
                let right_sql = right.to_sql(start + left_count);
                format!("({left_sql} OR {right_sql})")
            }
        }
    }

    /// Collect all bind values in order.
    pub fn binds(&self) -> Vec<Value> {
        match self {
            Expr::Compare { value, .. } => vec![value.clone()],
            Expr::IsNull { .. } => vec![],
            Expr::InList { values, .. } => values.clone(),
            Expr::And(left, right) | Expr::Or(left, right) => {
                let mut v = left.binds();
                v.extend(right.binds());
                v
            }
        }
    }

    /// Number of bind parameters this expression contributes.
    pub fn bind_count(&self) -> usize {
        match self {
            Expr::Compare { .. } => 1,
            Expr::IsNull { .. } => 0,
            Expr::InList { values, .. } => values.len(),
            Expr::And(left, right) | Expr::Or(left, right) => {
                left.bind_count() + right.bind_count()
            }
        }
    }

    /// Combine with AND.
    pub fn and(self, other: Expr) -> Expr {
        Expr::And(Box::new(self), Box::new(other))
    }

    /// Combine with OR.
    pub fn or(self, other: Expr) -> Expr {
        Expr::Or(Box::new(self), Box::new(other))
    }
}

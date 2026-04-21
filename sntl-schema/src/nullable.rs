use crate::schema::Schema;
use crate::scope::{JoinKind, Scope};
use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, Ident, Value,
};

pub struct ExprContext<'a> {
    pub schema: &'a Schema,
    pub scope: &'a Scope,
    pub strict: bool,
}

#[derive(Debug, Clone)]
pub struct NullabilityInfo {
    pub nullable: bool,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

pub fn infer_expr_nullability(expr: &Expr, ctx: &ExprContext) -> NullabilityInfo {
    match expr {
        Expr::Value(vws) => match &vws.value {
            Value::Null => NullabilityInfo {
                nullable: true,
                confidence: Confidence::High,
            },
            _ => NullabilityInfo {
                nullable: false,
                confidence: Confidence::High,
            },
        },

        // Typed cast — nullability follows inner
        Expr::Cast { expr, .. } => infer_expr_nullability(expr, ctx),

        // Column reference
        Expr::Identifier(ident) => resolve_identifier(std::slice::from_ref(ident), ctx),
        Expr::CompoundIdentifier(parts) => resolve_identifier(parts, ctx),

        // IS NULL / IS NOT NULL → boolean non-null
        Expr::IsNull(_)
        | Expr::IsNotNull(_)
        | Expr::IsTrue(_)
        | Expr::IsFalse(_)
        | Expr::Exists { .. } => NullabilityInfo {
            nullable: false,
            confidence: Confidence::High,
        },

        // CASE expressions
        Expr::Case {
            conditions,
            else_result,
            ..
        } => {
            let any_null = conditions
                .iter()
                .any(|cw| infer_expr_nullability(&cw.result, ctx).nullable)
                || else_result.is_none()
                || else_result
                    .as_ref()
                    .map(|e| infer_expr_nullability(e, ctx).nullable)
                    .unwrap_or(false);
            NullabilityInfo {
                nullable: any_null,
                confidence: Confidence::Medium,
            }
        }

        // Function call
        Expr::Function(func) => infer_function_nullability(func, ctx),

        // Binary op — nullable if either side is
        Expr::BinaryOp { left, right, .. } => {
            let l = infer_expr_nullability(left, ctx);
            let r = infer_expr_nullability(right, ctx);
            NullabilityInfo {
                nullable: l.nullable || r.nullable,
                confidence: min_confidence(l.confidence, r.confidence),
            }
        }

        _ => NullabilityInfo {
            nullable: ctx.strict,
            confidence: Confidence::Low,
        },
    }
}

fn resolve_identifier(parts: &[Ident], ctx: &ExprContext) -> NullabilityInfo {
    let (alias, column) = match parts.len() {
        1 => (None, parts[0].value.as_str()),
        2 => (Some(parts[0].value.as_str()), parts[1].value.as_str()),
        _ => {
            return NullabilityInfo {
                nullable: ctx.strict,
                confidence: Confidence::Low,
            };
        }
    };

    let table_ref = match alias {
        Some(a) => ctx.scope.resolve_alias(a),
        None => {
            let hits: Vec<_> = ctx
                .scope
                .tables
                .iter()
                .filter(|t| ctx.schema.find_column(&t.table_name, column).is_some())
                .collect();
            if hits.len() == 1 {
                Some(hits[0])
            } else {
                None
            }
        }
    };

    let table_ref = match table_ref {
        Some(tr) => tr,
        None => {
            return NullabilityInfo {
                nullable: ctx.strict,
                confidence: Confidence::Low,
            };
        }
    };

    let col = match ctx.schema.find_column(&table_ref.table_name, column) {
        Some(c) => c,
        None => {
            return NullabilityInfo {
                nullable: ctx.strict,
                confidence: Confidence::Low,
            };
        }
    };

    let mut nullable = col.nullable;
    if matches!(
        table_ref.join_kind,
        JoinKind::LeftForcedNullable | JoinKind::RightForcedNullable | JoinKind::FullForcedNullable
    ) {
        nullable = true;
    }
    NullabilityInfo {
        nullable,
        confidence: Confidence::High,
    }
}

fn extract_arg_exprs(func: &Function) -> Vec<&Expr> {
    let list = match &func.args {
        FunctionArguments::List(l) => l,
        _ => return vec![],
    };
    list.args
        .iter()
        .filter_map(|a| match a {
            FunctionArg::Named {
                arg: FunctionArgExpr::Expr(e),
                ..
            } => Some(e),
            FunctionArg::ExprNamed {
                arg: FunctionArgExpr::Expr(e),
                ..
            } => Some(e),
            FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => Some(e),
            _ => None,
        })
        .collect()
}

fn infer_function_nullability(func: &Function, ctx: &ExprContext) -> NullabilityInfo {
    let name = func.name.to_string().to_lowercase();
    let args = extract_arg_exprs(func);

    match name.as_str() {
        "coalesce" => {
            let any_non_null = args.iter().any(|a| !infer_expr_nullability(a, ctx).nullable);
            NullabilityInfo {
                nullable: !any_non_null,
                confidence: Confidence::Medium,
            }
        }
        "nullif" => NullabilityInfo {
            nullable: true,
            confidence: Confidence::High,
        },
        "count" => NullabilityInfo {
            nullable: false,
            confidence: Confidence::High,
        },
        "sum" | "avg" | "min" | "max" => NullabilityInfo {
            nullable: true,
            confidence: Confidence::High,
        },
        "row_number" | "rank" | "dense_rank" => NullabilityInfo {
            nullable: false,
            confidence: Confidence::High,
        },
        "lag" | "lead" => NullabilityInfo {
            nullable: true,
            confidence: Confidence::High,
        },
        _ => NullabilityInfo {
            nullable: ctx.strict,
            confidence: Confidence::Low,
        },
    }
}

fn min_confidence(a: Confidence, b: Confidence) -> Confidence {
    use Confidence::*;
    match (a, b) {
        (Low, _) | (_, Low) => Low,
        (Medium, _) | (_, Medium) => Medium,
        _ => High,
    }
}

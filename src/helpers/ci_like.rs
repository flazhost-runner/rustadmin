//! Case-insensitive `LIKE` across dialects (mirrors NodeAdmin `ciLike()`).
//!
//! MySQL is case-insensitive by default, but Postgres/SQLite are not — so we never write a
//! raw `LIKE :param`. Instead we lower **both sides**: `LOWER(col) LIKE LOWER('%value%')`.
//! Returns a `SimpleExpr` usable directly in `QueryFilter::filter`.

use sea_orm::sea_query::{Expr, Func, SimpleExpr};
use sea_orm::{ColumnTrait, IntoSimpleExpr};

/// Build `LOWER(col) LIKE '%lower(value)%'` for any entity column.
pub fn ci_like<C: ColumnTrait>(col: C, value: &str) -> SimpleExpr {
    let pattern = format!("%{}%", value.to_lowercase());
    Expr::expr(Func::lower(col.into_simple_expr())).like(pattern)
}

//! SQL placeholder utilities for cross-database compatibility.
//!
//! SQLite uses `?` for all bind parameters; PostgreSQL uses numbered
//! parameters `$1`, `$2`, etc.  These helpers let you write SQL once and
//! adapt it to either dialect automatically.
//!
//! # Quick reference
//!
//! | Call | SQLite result | PostgreSQL result |
//! |------|---------------|-------------------|
//! | `ph(pg, 1)` | `"?"` | `"$1"` |
//! | `phs(pg, 3)` | `"?, ?, ?"` | `"$1, $2, $3"` |
//! | `adapt_sql(pg, "WHERE id = ? AND x = ?")` | unchanged | `"WHERE id = $1 AND x = $2"` |

/// Return a single placeholder for bind parameter number `n`.
///
/// * PostgreSQL: `$n`
/// * SQLite: `?`
pub fn ph(is_postgres: bool, n: usize) -> String {
    if is_postgres {
        format!("${}", n)
    } else {
        "?".to_string()
    }
}

/// Return `count` comma-separated placeholders.
///
/// * PostgreSQL: `$1, $2, ..., $count`
/// * SQLite: `?, ?, ...`
pub fn phs(is_postgres: bool, count: usize) -> String {
    if is_postgres {
        (1..=count)
            .map(|n| format!("${}", n))
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        vec!["?"; count].join(", ")
    }
}

/// Adapt a `?`-style SQL string to the target database dialect.
///
/// For PostgreSQL every `?` is replaced with `$1`, `$2`, … in left-to-right
/// order.  For SQLite the string is returned unchanged.
pub fn adapt_sql(is_postgres: bool, sql: &str) -> String {
    if !is_postgres {
        return sql.to_string();
    }
    let mut counter = 0usize;
    let mut result = String::with_capacity(sql.len() + 16);
    for ch in sql.chars() {
        if ch == '?' {
            counter += 1;
            result.push_str(&format!("${}", counter));
        } else {
            result.push(ch);
        }
    }
    result
}

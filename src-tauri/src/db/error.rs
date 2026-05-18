use rusqlite::ErrorCode;

/// Check if a rusqlite error is a UNIQUE constraint violation.
fn is_unique_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(failure, Some(msg))
            if matches!(failure.code, ErrorCode::ConstraintViolation)
                && msg.contains("UNIQUE constraint failed")
    )
}

/// Map a rusqlite error from an INSERT operation to a user-friendly anyhow error.
/// Detects UNIQUE constraint violations and returns a clear message.
#[allow(clippy::needless_pass_by_value)]
pub fn on_insert(err: rusqlite::Error, label: &str) -> anyhow::Error {
    if is_unique_violation(&err) {
        anyhow::anyhow!("{label}已存在，请使用不同的名称")
    } else {
        anyhow::anyhow!("{label}添加失败: {err}")
    }
}

/// Map a rusqlite error from an UPDATE operation to a user-friendly anyhow error.
/// Detects UNIQUE constraint violations (e.g., renaming to an existing name).
#[allow(clippy::needless_pass_by_value)]
pub fn on_update(err: rusqlite::Error, label: &str) -> anyhow::Error {
    if is_unique_violation(&err) {
        anyhow::anyhow!("{label}已存在，请使用不同的名称")
    } else {
        anyhow::anyhow!("{label}更新失败: {err}")
    }
}

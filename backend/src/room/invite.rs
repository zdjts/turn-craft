use rand::Rng;

/// 邀请码生成和管理
pub async fn get_or_create(pool: &sqlx::SqlitePool, room_id: &str) -> Result<String, crate::error::AppError> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT invite_code FROM rooms WHERE room_id = ? AND invite_code IS NOT NULL"
    )
    .bind(room_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| crate::error::AppError::Internal(e.into()))?
    .flatten();

    if let Some(code) = existing {
        return Ok(code);
    }

    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::from_entropy();
    let new: String = (0..8).map(|_| {
        let idx = rng.gen_range(0..36);
        char::from_digit(idx, 36).unwrap()
    }).collect();

    sqlx::query("UPDATE rooms SET invite_code = ? WHERE room_id = ?")
        .bind(&new)
        .bind(room_id)
        .execute(pool)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.into()))?;

    Ok(new)
}

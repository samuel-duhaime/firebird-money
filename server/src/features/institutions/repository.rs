use sqlx::{PgConnection, PgPool};

use super::defaults::DEFAULT_INSTITUTIONS;
use super::model::{Institution, InstitutionPatch, NewInstitution};
use crate::features::transactions::same_institution_name;

/// Ensures every entry in `DEFAULT_INSTITUTIONS` exists as a common institution (`household_id IS
/// NULL`). Called once at server startup; idempotent, same as `merchants::repository::seed_defaults`.
pub async fn seed_defaults(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut query_builder =
        sqlx::QueryBuilder::new("INSERT INTO institutions (name, website_url, aliases) ");
    query_builder.push_values(DEFAULT_INSTITUTIONS.iter(), |mut row, default| {
        row.push_bind(default.name)
            .push_bind(default.website_url)
            .push_bind(default.aliases);
    });
    query_builder.push(" ON CONFLICT (name) WHERE household_id IS NULL DO NOTHING");
    query_builder.build().execute(pool).await?;
    Ok(())
}

/// Every column of an `Institution`, selected from a row aliased `i`, with `account_count` counted
/// against household `$1`.
const SELECT_COLUMNS: &str = "
    i.id, i.household_id, i.name, i.aliases, i.website_url, i.logo_url, i.created_at,
    (SELECT COUNT(*) FROM accounts WHERE institution_id = i.id AND household_id = $1) AS account_count";

/// Lists institutions visible to `household_id` (the shared common set, plus this household's own),
/// A-Z by name.
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<Institution>, sqlx::Error> {
    sqlx::query_as::<_, Institution>(&format!(
        "SELECT {SELECT_COLUMNS}
         FROM institutions i
         WHERE i.household_id = $1 OR i.household_id IS NULL
         ORDER BY lower(i.name)"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single institution visible to `household_id`, or `None` if it doesn't exist or
/// belongs to a different household.
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<Option<Institution>, sqlx::Error> {
    sqlx::query_as::<_, Institution>(&format!(
        "SELECT {SELECT_COLUMNS}
         FROM institutions i
         WHERE i.id = $2 AND (i.household_id = $1 OR i.household_id IS NULL)"
    ))
    .bind(household_id)
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Inserts a new custom institution owned by `household_id` and returns the created row.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    new_institution: &NewInstitution,
) -> Result<Institution, sqlx::Error> {
    sqlx::query_as::<_, Institution>(&format!(
        "WITH inserted AS (
            INSERT INTO institutions (household_id, name, website_url) VALUES ($1, $2, $3)
            RETURNING *
         )
         SELECT {SELECT_COLUMNS} FROM inserted i"
    ))
    .bind(household_id)
    .bind(&new_institution.name)
    .bind(&new_institution.website_url)
    .fetch_one(pool)
    .await
}

/// Applies a partial update, scoped to `household_id`. `website_url` follows a three-state rule:
/// `None` leaves it unchanged, `Some("")` clears it, `Some(url)` sets it. Returns `None` if the id
/// doesn't exist, belongs to a different household, or is a common institution
/// (`household_id = $1` never matches a `NULL` row).
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    patch: &InstitutionPatch,
) -> Result<Option<Institution>, sqlx::Error> {
    sqlx::query_as::<_, Institution>(&format!(
        "WITH updated AS (
            UPDATE institutions
            SET name = COALESCE($3, name),
                website_url = CASE WHEN $4::text IS NULL THEN website_url ELSE NULLIF($4, '') END
            WHERE id = $2 AND household_id = $1
            RETURNING *
         )
         SELECT {SELECT_COLUMNS} FROM updated i"
    ))
    .bind(household_id)
    .bind(id)
    .bind(&patch.name)
    .bind(&patch.website_url)
    .fetch_optional(pool)
    .await
}

/// Deletes a custom institution, scoped to `household_id`. Returns `false` if the id didn't exist,
/// belongs to a different household, or is a common institution. Blocked by a foreign-key violation
/// while any account is still held there.
pub async fn delete(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM institutions WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// `true` if `institution_id` is a common institution or belongs to `household_id`. Needed wherever
/// a client supplies a raw `institution_id` (accounts::repository's create/update), since there's no
/// composite FK to enforce this at the DB level.
pub async fn is_visible_to_household(
    pool: &PgPool,
    household_id: i32,
    institution_id: i32,
) -> Result<bool, sqlx::Error> {
    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(
            SELECT 1 FROM institutions
            WHERE id = $1 AND (household_id = $2 OR household_id IS NULL)
         )",
    )
    .bind(institution_id)
    .bind(household_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Finds an institution visible to `household_id` (common or its own) that is this one — its name
/// or one of its aliases equal to `name` (ignoring case), else appearing in `name` as whole words or
/// the other way around ("Caisse Desjardins du Plateau" → "Desjardins"), the longest such name
/// winning — else creates a custom one named `name`. Returns `(id, created)`. Used by statement
/// import, for an account a statement names that's new.
pub async fn find_or_create_named(
    conn: &mut PgConnection,
    household_id: i32,
    name: &str,
) -> Result<(i32, bool), sqlx::Error> {
    // Every visible institution's name and aliases, the household's own first so they win ties.
    let candidates: Vec<(i32, String)> = sqlx::query_as(
        "SELECT id, candidate FROM (
            SELECT id, household_id, name AS candidate FROM institutions
            WHERE household_id = $1 OR household_id IS NULL
            UNION ALL
            SELECT id, household_id, unnest(aliases) FROM institutions
            WHERE household_id = $1 OR household_id IS NULL
         ) c
         ORDER BY household_id NULLS LAST, id",
    )
    .bind(household_id)
    .fetch_all(&mut *conn)
    .await?;

    let exact = candidates
        .iter()
        .find(|(_, candidate)| candidate.to_lowercase() == name.to_lowercase())
        .map(|(id, _)| *id);
    let fuzzy = || {
        // Reversed: `max_by_key` keeps the last of equal maxima, so this keeps the household's own.
        candidates
            .iter()
            .rev()
            .filter(|(_, candidate)| same_institution_name(candidate, name))
            .max_by_key(|(_, candidate)| candidate.len())
            .map(|(id, _)| *id)
    };
    if let Some(id) = exact.or_else(fuzzy) {
        return Ok((id, false));
    }

    // A no-op `DO UPDATE` to get the id back on a race with itself; `xmax = 0` only holds for a
    // freshly inserted row.
    sqlx::query_as(
        "INSERT INTO institutions (household_id, name) VALUES ($1, $2)
         ON CONFLICT (household_id, name) WHERE household_id IS NOT NULL
         DO UPDATE SET name = EXCLUDED.name
         RETURNING id, (xmax = 0)",
    )
    .bind(household_id)
    .bind(name)
    .fetch_one(&mut *conn)
    .await
}

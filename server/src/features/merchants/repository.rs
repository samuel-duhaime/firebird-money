use sqlx::{PgConnection, PgPool};

use super::defaults::DEFAULT_MERCHANTS;
use super::model::{Merchant, MerchantFilter, MerchantPatch, MerchantSortOrder, NewMerchant};

/// Ensures every name in `DEFAULT_MERCHANTS` exists as a common merchant (`household_id IS NULL`).
/// Called once at server startup, not at household creation — common merchants are shared singleton
/// rows, not per-household copies, so there's nothing to seed per household. Idempotent (`ON
/// CONFLICT ... DO NOTHING` against `idx_merchants_common_name`), so it's safe to run on every
/// restart, including when `DEFAULT_MERCHANTS` has grown since the last one.
pub async fn seed_defaults(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut query_builder = sqlx::QueryBuilder::new("INSERT INTO merchants (name, aliases) ");
    query_builder.push_values(DEFAULT_MERCHANTS.iter(), |mut row, default| {
        row.push_bind(default.name).push_bind(default.aliases);
    });
    query_builder.push(" ON CONFLICT (name) WHERE household_id IS NULL DO NOTHING");
    query_builder.build().execute(pool).await?;
    Ok(())
}

const SELECT_COLUMNS: &str = "id, household_id, name, aliases, logo_url, created_at";

/// The category with the most of this household's transactions for a given merchant, ties broken
/// by whichever was used most recently (highest `date`, then highest `id` as a stable tie-breaker
/// for same-day transactions — same convention `transactions::repository::list` uses). `NULL` until
/// the household has at least one transaction with that merchant. Always recomputed, never stored —
/// referencing `m.id`/`$1` (the merchant row and `household_id`) from whichever outer query embeds
/// it, same pattern as the `transaction_count` subquery below.
const RECOMMENDED_CATEGORY_SUBQUERY: &str = "
    (SELECT category_id FROM transactions
     WHERE merchant_id = m.id AND household_id = $1
     GROUP BY category_id
     ORDER BY COUNT(*) DESC, MAX(date) DESC, MAX(id) DESC
     LIMIT 1)";

/// Lists merchants visible to `household_id` (the shared common set, plus this household's own),
/// each with how many of the household's transactions currently carry it, sorted per `filter.order`
/// (most-used first by default).
pub async fn list(
    pool: &PgPool,
    household_id: i32,
    filter: &MerchantFilter,
) -> Result<Vec<Merchant>, sqlx::Error> {
    let order_by = match filter.order {
        None | Some(MerchantSortOrder::TransactionCount) => "transaction_count DESC, name",
        Some(MerchantSortOrder::Alphabetical) => "name",
    };

    sqlx::query_as::<_, Merchant>(&format!(
        "SELECT {SELECT_COLUMNS},
                {RECOMMENDED_CATEGORY_SUBQUERY} AS recommended_category_id,
                (SELECT COUNT(*) FROM transactions
                 WHERE merchant_id = m.id AND household_id = $1) AS transaction_count
         FROM merchants m
         WHERE m.household_id = $1 OR m.household_id IS NULL
         ORDER BY {order_by}"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single merchant visible to `household_id`, or `None` if it doesn't exist or belongs to
/// a different household.
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<Option<Merchant>, sqlx::Error> {
    sqlx::query_as::<_, Merchant>(&format!(
        "SELECT {SELECT_COLUMNS},
                {RECOMMENDED_CATEGORY_SUBQUERY} AS recommended_category_id,
                (SELECT COUNT(*) FROM transactions
                 WHERE merchant_id = m.id AND household_id = $1) AS transaction_count
         FROM merchants m
         WHERE m.id = $2 AND (m.household_id = $1 OR m.household_id IS NULL)"
    ))
    .bind(household_id)
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Inserts a new custom merchant owned by `household_id` and returns the created row.
/// `recommended_category_id` is always `None` on creation — there's no transaction history yet.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    new_merchant: &NewMerchant,
) -> Result<Merchant, sqlx::Error> {
    sqlx::query_as::<_, Merchant>(&format!(
        "WITH inserted AS (
            INSERT INTO merchants (household_id, name) VALUES ($1, $2) RETURNING *
         )
         SELECT {SELECT_COLUMNS},
                {RECOMMENDED_CATEGORY_SUBQUERY} AS recommended_category_id,
                (SELECT COUNT(*) FROM transactions
                 WHERE merchant_id = m.id AND household_id = $1) AS transaction_count
         FROM inserted m"
    ))
    .bind(household_id)
    .bind(&new_merchant.name)
    .fetch_one(pool)
    .await
}

/// Applies a partial update (only `Some` fields change), scoped to `household_id`, and returns the
/// updated row, or `None` if the id doesn't exist (including when it belongs to a different
/// household, or is a common merchant — `household_id = $1` can never match a `NULL` row, so this
/// structurally can't rename/delete a common merchant, no special-casing needed).
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    patch: &MerchantPatch,
) -> Result<Option<Merchant>, sqlx::Error> {
    sqlx::query_as::<_, Merchant>(&format!(
        "WITH updated AS (
            UPDATE merchants
            SET name = COALESCE($3, name)
            WHERE id = $2 AND household_id = $1
            RETURNING *
         )
         SELECT {SELECT_COLUMNS},
                {RECOMMENDED_CATEGORY_SUBQUERY} AS recommended_category_id,
                (SELECT COUNT(*) FROM transactions
                 WHERE merchant_id = m.id AND household_id = $1) AS transaction_count
         FROM updated m"
    ))
    .bind(household_id)
    .bind(id)
    .bind(&patch.name)
    .fetch_optional(pool)
    .await
}

/// Deletes a merchant by id, scoped to `household_id`. Returns `true` if a row was deleted, `false`
/// if the id didn't exist (including when it belongs to a different household, or is a common
/// merchant). Blocked by a foreign-key violation while any transaction still carries it
/// (`transactions.merchant_id` has no `ON DELETE CASCADE`).
pub async fn delete(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM merchants WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// `true` if `merchant_id` is a common merchant or belongs to `household_id`. Needed wherever a
/// client supplies a raw `merchant_id` directly (transactions::repository's create/update/
/// bulk_update): unlike `category_id`, there's no composite FK to enforce this at the DB level,
/// since a common merchant's `household_id` is `NULL`, not the caller's household.
pub async fn is_visible_to_household(
    pool: &PgPool,
    household_id: i32,
    merchant_id: i32,
) -> Result<bool, sqlx::Error> {
    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(
            SELECT 1 FROM merchants WHERE id = $1 AND (household_id = $2 OR household_id IS NULL)
         )",
    )
    .bind(merchant_id)
    .bind(household_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Whether `needle` occurs in `haystack` bounded by a non-alphanumeric character (or the start/end
/// of the string) on both sides — e.g. "td" matches "my td bank" but not "ltd" or "today". Without
/// this, short merchant names like "TD" or "Bell" would match substrings of unrelated words ("LTD",
/// "CAMPBELL").
fn contains_word_boundary(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    haystack.match_indices(needle).any(|(start, matched)| {
        let before_ok = haystack[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let after_ok = haystack[start + matched.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        before_ok && after_ok
    })
}

/// Pure matching logic, split out from `find_match` so it's unit-testable without a database:
/// case-insensitive, word-bounded containment of a candidate's `name` inside `statement` (see
/// `contains_word_boundary`), longest candidate name wins on a tie (most specific match, and — since
/// a name can't be longer than the text it's found inside — an exact match always wins outright).
fn best_match(statement: &str, candidates: &[(i32, String)]) -> Option<i32> {
    let lower_statement = statement.to_lowercase();
    candidates
        .iter()
        .filter(|(_, name)| {
            !name.is_empty() && contains_word_boundary(&lower_statement, &name.to_lowercase())
        })
        .max_by_key(|(_, name)| name.len())
        .map(|(id, _)| *id)
}

/// Case-insensitive substring match against every merchant visible to `household_id` (common + this
/// household's own custom ones) — see `best_match`. Each merchant contributes one candidate per
/// `name`/alias (see `DefaultMerchant`), so an alias like "McDonalds" can match raw statement text
/// its real `name` ("McDonald's") wouldn't.
async fn find_match(
    conn: &mut PgConnection,
    household_id: i32,
    statement: &str,
) -> Result<Option<i32>, sqlx::Error> {
    let candidates: Vec<(i32, String)> = sqlx::query_as(
        "SELECT id, name FROM merchants WHERE household_id = $1 OR household_id IS NULL
         UNION ALL
         SELECT id, unnest(aliases) FROM merchants WHERE household_id = $1 OR household_id IS NULL",
    )
    .bind(household_id)
    .fetch_all(&mut *conn)
    .await?;

    Ok(best_match(statement, &candidates))
}

/// The entry point `transactions::repository::create` calls when a caller gives `original_statement`
/// without an explicit `merchant_id`: matches an existing merchant first, else creates a new custom
/// merchant named exactly `statement` for `household_id`. `ON CONFLICT ... DO UPDATE` (rather than
/// `DO NOTHING`) is a no-op update used purely to get the id back on a race with itself (two
/// concurrent imports creating the same new merchant name).
pub async fn resolve_or_create(
    conn: &mut PgConnection,
    household_id: i32,
    statement: &str,
) -> Result<i32, sqlx::Error> {
    if let Some(id) = find_match(conn, household_id, statement).await? {
        return Ok(id);
    }

    let (id,): (i32,) = sqlx::query_as(
        "INSERT INTO merchants (household_id, name) VALUES ($1, $2)
         ON CONFLICT (household_id, name) WHERE household_id IS NOT NULL
         DO UPDATE SET name = EXCLUDED.name
         RETURNING id",
    )
    .bind(household_id)
    .bind(statement)
    .fetch_one(&mut *conn)
    .await?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: i32, name: &str) -> (i32, String) {
        (id, name.to_string())
    }

    #[test]
    fn best_match_finds_a_substring_case_insensitively() {
        let candidates = vec![candidate(1, "Spotify")];
        assert_eq!(best_match("SPOTIFY P0A1B2", &candidates), Some(1));
    }

    #[test]
    fn best_match_returns_none_when_nothing_matches() {
        let candidates = vec![candidate(1, "Spotify")];
        assert_eq!(best_match("NotImportantMerchantName", &candidates), None);
    }

    #[test]
    fn best_match_prefers_the_longest_name_on_multiple_matches() {
        // "Amazon" matches, but "Amazon Prime" is a longer, more specific match for this text.
        let candidates = vec![candidate(1, "Amazon"), candidate(2, "Amazon Prime")];
        assert_eq!(best_match("AMAZON PRIME MEMBERSHIP", &candidates), Some(2));
    }

    #[test]
    fn best_match_ignores_candidates_with_a_blank_name() {
        let candidates = vec![candidate(1, "")];
        assert_eq!(best_match("anything", &candidates), None);
    }

    #[test]
    fn best_match_does_not_match_a_short_name_inside_an_unrelated_word() {
        let candidates = vec![candidate(1, "TD"), candidate(2, "Bell")];
        assert_eq!(best_match("PAYMENT TO LTD COMPANY", &candidates), None);
        assert_eq!(best_match("CAMPBELL SOUP CO", &candidates), None);
    }

    #[test]
    fn best_match_still_matches_a_short_name_at_a_word_boundary() {
        let candidates = vec![candidate(1, "TD")];
        assert_eq!(best_match("MY TD BANK VISA", &candidates), Some(1));
        assert_eq!(best_match("TD", &candidates), Some(1));
    }
}

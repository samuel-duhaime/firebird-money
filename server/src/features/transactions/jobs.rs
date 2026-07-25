//! In-memory tracking for async budget-file import jobs (`POST /transactions/import`). Jobs are
//! ephemeral by nature — a server restart also kills whatever `claude` subprocess was tracking
//! them — so there's no need to persist this to Postgres alongside actual transaction data.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;
use uuid::Uuid;

use super::model::{ImportJob, ImportJobStatus};

#[derive(Default)]
pub struct JobStore(Mutex<HashMap<Uuid, ImportJob>>);

impl JobStore {
    /// Creates a new job in `pending` status.
    pub fn create(&self, file_name: String) -> ImportJob {
        let now = Utc::now();
        let job = ImportJob {
            id: Uuid::new_v4(),
            status: ImportJobStatus::Pending,
            file_name,
            created_count: None,
            failed_count: None,
            skipped_count: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        };
        self.0.lock().unwrap().insert(job.id, job.clone());
        job
    }

    /// Fetches a job by id, or `None` if it doesn't exist (or the server has since restarted).
    pub fn get(&self, id: Uuid) -> Option<ImportJob> {
        self.0.lock().unwrap().get(&id).cloned()
    }

    /// Marks a job `running`. No-op if the job is unknown.
    pub fn mark_running(&self, id: Uuid) {
        if let Some(job) = self.0.lock().unwrap().get_mut(&id) {
            job.status = ImportJobStatus::Running;
            job.updated_at = Utc::now();
        }
    }

    /// Moves a job to a terminal status (`Succeeded`/`Failed`) with its final counts/error
    /// message. No-ops if the job is unknown or already terminal, so the unattended subprocess's
    /// own report can't be clobbered by the server's fallback failure handler racing behind it.
    #[allow(clippy::too_many_arguments)]
    pub fn complete(
        &self,
        id: Uuid,
        status: ImportJobStatus,
        created_count: Option<i32>,
        failed_count: Option<i32>,
        skipped_count: Option<i32>,
        error_message: Option<String>,
    ) {
        let mut jobs = self.0.lock().unwrap();
        let Some(job) = jobs.get_mut(&id) else {
            return;
        };
        if matches!(
            job.status,
            ImportJobStatus::Succeeded | ImportJobStatus::Failed
        ) {
            return;
        }
        job.status = status;
        job.created_count = created_count;
        job.failed_count = failed_count;
        job.skipped_count = skipped_count;
        job.error_message = error_message;
        job.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_none_for_unknown_id() {
        let store = JobStore::default();
        assert!(store.get(Uuid::new_v4()).is_none());
    }

    #[test]
    fn create_then_get_round_trips() {
        let store = JobStore::default();
        let job = store.create("statement.csv".to_string());

        assert_eq!(job.status, ImportJobStatus::Pending);
        assert_eq!(store.get(job.id).unwrap().status, ImportJobStatus::Pending);
    }

    #[test]
    fn mark_running_updates_status() {
        let store = JobStore::default();
        let job = store.create("statement.csv".to_string());

        store.mark_running(job.id);

        assert_eq!(store.get(job.id).unwrap().status, ImportJobStatus::Running);
    }

    #[test]
    fn complete_sets_terminal_status_and_counts() {
        let store = JobStore::default();
        let job = store.create("statement.csv".to_string());

        store.complete(
            job.id,
            ImportJobStatus::Succeeded,
            Some(3),
            Some(1),
            Some(2),
            None,
        );

        let updated = store.get(job.id).unwrap();
        assert_eq!(updated.status, ImportJobStatus::Succeeded);
        assert_eq!(updated.created_count, Some(3));
        assert_eq!(updated.failed_count, Some(1));
        assert_eq!(updated.skipped_count, Some(2));
    }

    #[test]
    fn complete_does_not_overwrite_a_terminal_job() {
        let store = JobStore::default();
        let job = store.create("statement.csv".to_string());
        store.complete(
            job.id,
            ImportJobStatus::Succeeded,
            Some(3),
            None,
            None,
            None,
        );

        // The server's own fallback failure handler racing in after the skill already reported
        // success shouldn't clobber it.
        store.complete(
            job.id,
            ImportJobStatus::Failed,
            None,
            None,
            None,
            Some("subprocess exited".to_string()),
        );

        let updated = store.get(job.id).unwrap();
        assert_eq!(updated.status, ImportJobStatus::Succeeded);
        assert_eq!(updated.created_count, Some(3));
    }
}

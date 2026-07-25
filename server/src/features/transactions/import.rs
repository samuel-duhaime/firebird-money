//! Drives an unattended `claude` subprocess through the `budget-file-to-transaction` skill for
//! `POST /transactions/import`. The subprocess talks to this same server's API directly (see the
//! skill's "Unattended mode" section); this module only stages the uploaded file, launches the
//! subprocess with a narrow tool allowlist, and falls back to marking the job failed if the
//! subprocess never reports its own result.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;
use uuid::Uuid;

use super::jobs::JobStore;
use super::model::ImportJobStatus;

/// Local address the server binds to (see `main.rs`) and that the subprocess calls back into.
const SERVER_ADDR: &str = "127.0.0.1:3055";

/// How much of the subprocess's combined stdout/stderr to keep as `error_message` when it fails
/// (or exits without reporting a result), so a runaway process can't bloat the database row.
const MAX_ERROR_MESSAGE_LEN: usize = 4000;

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The repo root, where `.claude/skills/` lives — the subprocess's cwd must be here (or a
/// descendant) for `/budget-file-to-transaction` to resolve.
fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("server crate is nested under the repo root")
        .to_path_buf()
}

/// Directory a given job's uploaded file is staged in. Lives under `server/data/transactions/
/// inputs/`, which is already gitignored for this exact purpose.
pub fn upload_dir(job_id: Uuid) -> PathBuf {
    manifest_dir()
        .join("data/transactions/inputs")
        .join(job_id.to_string())
}

/// Strips any directory components from an untrusted upload filename, keeping only the basename,
/// so it can't be used for path traversal when staged to disk.
pub fn sanitize_filename(name: &str) -> String {
    Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("upload")
        .to_string()
}

/// Builds the unattended `claude` invocation: no `--dangerously-skip-permissions`, instead a
/// pre-approved allowlist scoped to reading the uploaded file and calling this server's three
/// import-related endpoints — anything else has no TTY to prompt on, so it's denied automatically.
fn build_command(job_id: Uuid, file_path: &Path) -> Command {
    let upload_dir = upload_dir(job_id);
    let prompt = format!(
        "/budget-file-to-transaction Import the file at {} with job_id={job_id}.",
        file_path.display()
    );
    // `*` right after `curl` tolerates incidental flags (e.g. `-s`) the model adds out of habit —
    // a rigid `curl http://...` prefix broke on the first real run when it wrote `curl -s http://...`.
    let allowed_tools = format!(
        "Read({upload_dir}/*) \
         Bash(curl*http://{SERVER_ADDR}/categories*) \
         Bash(curl*-X POST*http://{SERVER_ADDR}/transactions*) \
         Bash(curl*-X PATCH*http://{SERVER_ADDR}/transactions/import/jobs/*)",
        upload_dir = upload_dir.display(),
    );

    let mut command = Command::new("claude");
    command
        .current_dir(repo_root())
        .arg("-p")
        .arg(prompt)
        .arg("--tools")
        .arg("Read,Bash")
        .arg("--allowedTools")
        .arg(allowed_tools)
        .arg("--no-session-persistence")
        .arg("--output-format")
        .arg("text")
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Truncates `text` to the last `MAX_ERROR_MESSAGE_LEN` characters, so a runaway subprocess can't
/// bloat the stored error message.
fn truncate_tail(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= MAX_ERROR_MESSAGE_LEN {
        text.to_string()
    } else {
        text.chars()
            .skip(char_count - MAX_ERROR_MESSAGE_LEN)
            .collect()
    }
}

/// Runs the full unattended import: marks the job running, spawns the subprocess and waits for
/// it, then — only if the job is still `pending`/`running` afterward, meaning the skill's own
/// final `PATCH` never landed — marks it failed using the captured output. Always cleans up the
/// staged upload file.
pub async fn run_import(job_store: &JobStore, job_id: Uuid, file_path: PathBuf) {
    job_store.mark_running(job_id);

    let output = build_command(job_id, &file_path).output().await;

    let fallback_error = match &output {
        Ok(output) => {
            let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
            combined.push_str(&String::from_utf8_lossy(&output.stderr));
            if output.status.success() {
                truncate_tail(&format!(
                    "unattended import subprocess exited successfully but never reported a \
                     result; last output:\n{combined}"
                ))
            } else {
                truncate_tail(&format!(
                    "unattended import subprocess exited with {}; last output:\n{combined}",
                    output.status
                ))
            }
        }
        Err(e) => format!("failed to start unattended import subprocess: {e}"),
    };

    // No-op if the skill's own PATCH already moved the job to a terminal status.
    job_store.complete(
        job_id,
        ImportJobStatus::Failed,
        None,
        None,
        None,
        Some(fallback_error),
    );

    if let Err(e) = tokio::fs::remove_dir_all(upload_dir(job_id)).await {
        log::error!("failed to clean up import upload dir job_id={job_id} error={e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_keeps_a_plain_name() {
        assert_eq!(sanitize_filename("statement.csv"), "statement.csv");
    }

    #[test]
    fn sanitize_filename_strips_directory_components() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("/etc/passwd"), "passwd");
    }

    #[test]
    fn sanitize_filename_falls_back_for_empty_or_dot_only_input() {
        assert_eq!(sanitize_filename(""), "upload");
        assert_eq!(sanitize_filename(".."), "upload");
    }

    #[test]
    fn truncate_tail_keeps_short_text_unchanged() {
        assert_eq!(truncate_tail("short"), "short");
    }

    #[test]
    fn truncate_tail_keeps_only_the_last_chars_of_long_text() {
        let long = "a".repeat(MAX_ERROR_MESSAGE_LEN + 100);
        let truncated = truncate_tail(&long);
        assert_eq!(truncated.chars().count(), MAX_ERROR_MESSAGE_LEN);
        assert!(long.ends_with(&truncated));
    }
}

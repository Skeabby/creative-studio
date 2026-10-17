//! A tiny background-job runner for M2 installs: one job at a time, running on
//! a worker thread and reporting progress to the UI through a channel.
//!
//! The UI never blocks: [`Jobs::submit`] spawns the work and returns
//! immediately, and the caller drains [`JobEvent`]s once per frame exactly like
//! the app-lifecycle [`crate::runtime::Event`]s. A running job can be asked to
//! stop with [`Jobs::cancel`]; the worker observes the flag through its
//! [`JobContext`] and returns early.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// The phase a job is in, for the status line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Downloading,
    Verifying,
    Installing,
    Removing,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::Downloading => "Downloading",
            Stage::Verifying => "Verifying",
            Stage::Installing => "Installing",
            Stage::Removing => "Removing",
        }
    }
}

/// What the worker tells the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    /// A job began. `label` is human-readable (`Install Notes`).
    Started { id: String, label: String },
    /// The job moved to another phase.
    Stage { id: String, stage: Stage },
    /// Byte-level progress within the current phase.
    Progress {
        id: String,
        done: u64,
        total: Option<u64>,
    },
    /// The job completed successfully; `summary` is a short human line.
    Finished { id: String, summary: String },
    /// The job failed with a message to surface.
    Failed { id: String, message: String },
    /// The job stopped because it was cancelled.
    Cancelled { id: String },
}

/// The handle a running job uses to report progress and observe cancellation.
pub struct JobContext {
    id: String,
    tx: Sender<JobEvent>,
    cancel: Arc<AtomicBool>,
}

impl JobContext {
    /// Has the user asked this job to stop?
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// A clone of the cancellation flag, for APIs that take `&AtomicBool`
    /// (e.g. [`crate::net::download_to_file`]).
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }

    /// Announce a new phase.
    pub fn stage(&self, stage: Stage) {
        let _ = self.tx.send(JobEvent::Stage {
            id: self.id.clone(),
            stage,
        });
    }

    /// Report byte progress within the current phase.
    pub fn progress(&self, done: u64, total: Option<u64>) {
        let _ = self.tx.send(JobEvent::Progress {
            id: self.id.clone(),
            done,
            total,
        });
    }
}

/// The job runner. Cheap to construct; holds the worker channel and flags.
pub struct Jobs {
    tx: Sender<JobEvent>,
    cancel: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
}

impl Default for Jobs {
    fn default() -> Self {
        Self::new().0
    }
}

impl Jobs {
    /// Create a runner and the channel the UI must drain each frame.
    pub fn new() -> (Jobs, Receiver<JobEvent>) {
        let (tx, rx) = mpsc::channel();
        let jobs = Jobs {
            tx,
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
        };
        (jobs, rx)
    }

    /// Is a job currently running?
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }

    /// Submit a job. Returns `false` (and does nothing) if one is already
    /// running — the caller can tell the user to wait.
    pub fn submit<F>(&self, id: impl Into<String>, label: impl Into<String>, work: F) -> bool
    where
        F: FnOnce(JobContext) -> Result<String, String> + Send + 'static,
    {
        if self.busy.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.cancel.store(false, Ordering::SeqCst);
        let id = id.into();
        let label = label.into();
        let tx = self.tx.clone();
        let cancel = Arc::clone(&self.cancel);
        let busy = Arc::clone(&self.busy);
        thread::spawn(move || {
            let _ = tx.send(JobEvent::Started {
                id: id.clone(),
                label,
            });
            let ctx = JobContext {
                id: id.clone(),
                tx: tx.clone(),
                cancel: Arc::clone(&cancel),
            };
            let outcome = work(ctx);
            let event = match outcome {
                Ok(summary) => JobEvent::Finished { id, summary },
                Err(_) if cancel.load(Ordering::Relaxed) => JobEvent::Cancelled { id },
                Err(message) => JobEvent::Failed { id, message },
            };
            let _ = tx.send(event);
            busy.store(false, Ordering::SeqCst);
        });
        true
    }

    /// Ask the running job (if any) to stop. Cooperative: the worker returns at
    /// its next cancellation check.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(5);

    fn recv(rx: &Receiver<JobEvent>) -> JobEvent {
        rx.recv_timeout(TIMEOUT)
            .expect("an event within the timeout")
    }

    #[test]
    fn a_successful_job_reports_start_then_finish() {
        let (jobs, rx) = Jobs::new();
        assert!(jobs.submit("nucleus.notes", "Install Notes", |ctx| {
            ctx.stage(Stage::Installing);
            ctx.progress(10, Some(10));
            Ok("installed 1.2.0".into())
        }));
        assert_eq!(
            recv(&rx),
            JobEvent::Started {
                id: "nucleus.notes".into(),
                label: "Install Notes".into()
            }
        );
        assert_eq!(
            recv(&rx),
            JobEvent::Stage {
                id: "nucleus.notes".into(),
                stage: Stage::Installing
            }
        );
        assert_eq!(
            recv(&rx),
            JobEvent::Progress {
                id: "nucleus.notes".into(),
                done: 10,
                total: Some(10)
            }
        );
        assert_eq!(
            recv(&rx),
            JobEvent::Finished {
                id: "nucleus.notes".into(),
                summary: "installed 1.2.0".into()
            }
        );
        // The runner frees up once the job is done.
        for _ in 0..50 {
            if !jobs.is_busy() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("runner stayed busy after the job finished");
    }

    #[test]
    fn a_failing_job_reports_failure() {
        let (jobs, rx) = Jobs::new();
        jobs.submit("nucleus.notes", "Install Notes", |_ctx| {
            Err("hash mismatch".into())
        });
        let _ = recv(&rx); // Started
        assert_eq!(
            recv(&rx),
            JobEvent::Failed {
                id: "nucleus.notes".into(),
                message: "hash mismatch".into()
            }
        );
    }

    #[test]
    fn a_cancelled_job_reports_cancellation() {
        let (jobs, rx) = Jobs::new();
        jobs.submit("nucleus.notes", "Install Notes", |ctx| {
            // Simulate a long download that observes the flag.
            while !ctx.is_cancelled() {
                thread::sleep(Duration::from_millis(5));
            }
            Err("stopped".into())
        });
        let _ = recv(&rx); // Started
        jobs.cancel();
        assert_eq!(
            recv(&rx),
            JobEvent::Cancelled {
                id: "nucleus.notes".into()
            }
        );
    }

    #[test]
    fn only_one_job_runs_at_a_time() {
        let (jobs, rx) = Jobs::new();
        let (gate_tx, gate_rx) = mpsc::channel::<()>();
        assert!(jobs.submit("a", "A", move |_ctx| {
            let _ = gate_rx.recv_timeout(TIMEOUT);
            Ok("a done".into())
        }));
        let _ = recv(&rx); // Started for "a"
        // A second submission while the first holds the gate must be refused.
        assert!(!jobs.submit("b", "B", |_ctx| Ok("b done".into())));
        gate_tx.send(()).unwrap();
        assert_eq!(
            recv(&rx),
            JobEvent::Finished {
                id: "a".into(),
                summary: "a done".into()
            }
        );
    }

    #[test]
    fn stage_labels_are_stable() {
        assert_eq!(Stage::Downloading.label(), "Downloading");
        assert_eq!(Stage::Verifying.label(), "Verifying");
        assert_eq!(Stage::Installing.label(), "Installing");
        assert_eq!(Stage::Removing.label(), "Removing");
    }
}

// Background job tracking for `&`, `jobs`, `fg`, `bg`.
//
// Jobs are tracked as OS threads (each running the normal, blocking
// Executor logic, including any child.wait() calls) rather than as raw
// process handles with real POSIX process-group job control. That means
// there's no SIGTSTP/SIGCONT-based suspend-and-resume -- `bg` on a job
// that's already running (which is the only kind we ever create) just
// confirms it exists rather than actually resuming anything, and a job
// can't be signaled/killed through this table directly (use `kill` with
// the PID from `ps` for that). This is a deliberately small slice of real
// job control, scoped to the common case: run something without blocking
// the shell, list it, and optionally wait for it later.
use std::thread::JoinHandle;

pub struct Job {
    pub id: usize,
    pub description: String,
    handle: Option<JoinHandle<i32>>,
    finished_code: Option<i32>,
}

pub struct JobTable {
    jobs: Vec<Job>,
    next_id: usize,
}

impl JobTable {
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            next_id: 1,
        }
    }

    // Registers jobs spawned by an Executor run, announcing each with its
    // new job id.
    pub fn absorb(&mut self, spawned: Vec<(String, JoinHandle<i32>)>) {
        for (description, handle) in spawned {
            let id = self.next_id;
            self.next_id += 1;
            println!("[{id}] {description}");
            self.jobs.push(Job {
                id,
                description,
                handle: Some(handle),
                finished_code: None,
            });
        }
    }

    // Call before each prompt: reports jobs that finished since the last
    // check, mirroring a real shell's asynchronous "Done" notifications.
    pub fn announce_finished(&mut self) {
        for job in &mut self.jobs {
            if job.finished_code.is_none() && job.handle.as_ref().is_some_and(|h| h.is_finished()) {
                let code = job.handle.take().unwrap().join().unwrap_or(1);
                job.finished_code = Some(code);
                println!("[{}]+ Done ({code})   {}", job.id, job.description);
            }
        }
    }

    // `jobs` builtin: lists everything currently tracked, then drops any
    // job that had already finished (a completed job is reported once by
    // `jobs`, then disappears from later listings, same as bash).
    pub fn list_and_purge_done(&mut self) -> Vec<String> {
        self.announce_finished();
        let lines = self
            .jobs
            .iter()
            .map(|j| {
                let status = if j.finished_code.is_some() { "Done" } else { "Running" };
                format!("[{}]  {:<7} {}", j.id, status, j.description)
            })
            .collect();
        self.jobs.retain(|j| j.finished_code.is_none());
        lines
    }

    pub fn most_recent_id(&self) -> Option<usize> {
        self.jobs.last().map(|j| j.id)
    }

    // `fg`: waits for the job to finish (if it hasn't already) and returns
    // its exit code, removing it from the table.
    pub fn bring_to_foreground(&mut self, id: usize) -> Option<i32> {
        let idx = self.jobs.iter().position(|j| j.id == id)?;
        let mut job = self.jobs.remove(idx);
        Some(match job.finished_code {
            Some(code) => code,
            None => job.handle.take().unwrap().join().unwrap_or(1),
        })
    }

    pub fn contains(&self, id: usize) -> bool {
        self.jobs.iter().any(|j| j.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absorbing_and_listing_a_job() {
        let mut table = JobTable::new();
        table.absorb(vec![("sleep 1".to_string(), std::thread::spawn(|| 0))]);

        // Give the trivial thread a moment to finish so this is exercised
        // deterministically rather than relying on scheduler timing.
        std::thread::sleep(std::time::Duration::from_millis(50));

        let lines = table.list_and_purge_done();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("sleep 1"));
        // Listed once as Done, then purged.
        assert!(table.jobs.is_empty());
    }

    #[test]
    fn fg_waits_for_the_job_and_returns_its_exit_code() {
        let mut table = JobTable::new();
        table.absorb(vec![("exit-with-7".to_string(), std::thread::spawn(|| 7))]);
        let id = table.most_recent_id().unwrap();

        assert_eq!(table.bring_to_foreground(id), Some(7));
        assert!(!table.contains(id));
    }

    #[test]
    fn fg_on_an_unknown_id_returns_none() {
        let mut table = JobTable::new();
        assert_eq!(table.bring_to_foreground(999), None);
    }
}

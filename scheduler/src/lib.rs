//! Synchronous FIFO job scheduling for ARIA.

use std::collections::VecDeque;

type Job = Box<dyn FnOnce() + 'static>;

/// A synchronous first-in, first-out job scheduler.
#[derive(Default)]
pub struct Scheduler {
    pending: VecDeque<Job>,
}

impl Scheduler {
    /// Creates an empty scheduler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a job to the end of the pending queue.
    pub fn schedule(&mut self, job: impl FnOnce() + 'static) {
        self.pending.push_back(Box::new(job));
    }

    /// Executes all pending jobs synchronously in FIFO order.
    ///
    /// Returns the number of jobs executed.
    pub fn run_pending(&mut self) -> usize {
        let mut executed = 0;

        while let Some(job) = self.pending.pop_front() {
            job();
            executed += 1;
        }

        executed
    }
}

#[cfg(test)]
mod tests {
    use super::Scheduler;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn executes_jobs_in_fifo_order() {
        let execution_order = Rc::new(RefCell::new(Vec::new()));
        let mut scheduler = Scheduler::new();

        for job_id in 1..=3 {
            let execution_order = Rc::clone(&execution_order);
            scheduler.schedule(move || execution_order.borrow_mut().push(job_id));
        }

        assert_eq!(scheduler.run_pending(), 3);
        assert_eq!(*execution_order.borrow(), vec![1, 2, 3]);
    }

    #[test]
    fn removes_jobs_after_execution() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(|| {});

        assert_eq!(scheduler.run_pending(), 1);
        assert_eq!(scheduler.run_pending(), 0);
    }
}

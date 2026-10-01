//! A tiny fixed-size thread pool: jobs go in one queue, results come back
//! on another, nothing ever blocks the UI thread.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

/// Threads running `work` on every job sent to them.
pub struct Pool<J, R> {
    jobs: Sender<J>,
    done: Receiver<R>,
}

impl<J: Send + 'static, R: Send + 'static> Pool<J, R> {
    /// Example: `let pool = Pool::start(2, |n: u32| n * 2);`
    pub fn start(threads: usize, work: impl Fn(J) -> R + Send + Sync + 'static) -> Self {
        let (jobs, job_rx) = channel::<J>();
        let (done_tx, done) = channel::<R>();
        let (job_rx, work) = (Arc::new(Mutex::new(job_rx)), Arc::new(work));
        for _ in 0..threads.max(1) {
            let (rx, tx, work) = (job_rx.clone(), done_tx.clone(), work.clone());
            thread::spawn(move || serve(&rx, &tx, &*work));
        }
        Self { jobs, done }
    }

    /// Queues a job (silently dropped if the pool has shut down).
    pub fn send(&self, job: J) {
        let _ = self.jobs.send(job);
    }

    /// Finished results, without blocking.
    pub fn finished(&self) -> Vec<R> {
        self.done.try_iter().collect()
    }
}

fn serve<J, R>(jobs: &Mutex<Receiver<J>>, done: &Sender<R>, work: &dyn Fn(J) -> R) {
    loop {
        let job = match jobs.lock() {
            Ok(rx) => rx.recv(),
            Err(_) => return,
        };
        let Ok(job) = job else { return };
        if done.send(work(job)).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn runs_every_job() {
        let pool = Pool::start(3, |n: u32| n * 2);
        (0..10).for_each(|n| pool.send(n));
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut got = Vec::new();
        while got.len() < 10 && Instant::now() < deadline {
            got.extend(pool.finished());
            thread::sleep(Duration::from_millis(1));
        }
        got.sort();
        assert_eq!(got, (0..10).map(|n| n * 2).collect::<Vec<_>>());
    }
}

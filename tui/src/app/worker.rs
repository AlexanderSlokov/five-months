//! Background chunk generation, so scrolling never waits for a mountain to
//! be drawn (a chunk takes ~30 ms).

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::world::planner::Planner;
use crate::world::scenes::{Scene, chunk_scenes};

const THREADS: usize = 4;

/// A request: which world (`epoch`) and which chunk.
struct Job {
    epoch: u64,
    planner: Planner,
    chunk: i64,
}

/// A finished chunk.
pub struct Done {
    pub epoch: u64,
    pub chunk: i64,
    pub scenes: Vec<Scene>,
}

/// Pool of generator threads fed through one queue.
pub struct ChunkWorkers {
    jobs: Sender<Job>,
    done: Receiver<Done>,
    pending: HashSet<(u64, i64)>,
}

impl ChunkWorkers {
    /// Starts the pool. Example: `let mut w = ChunkWorkers::start();`
    pub fn start() -> Self {
        let (jobs, job_rx) = channel::<Job>();
        let (done_tx, done) = channel::<Done>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        for _ in 0..THREADS {
            let (rx, tx) = (job_rx.clone(), done_tx.clone());
            thread::spawn(move || work(&rx, &tx));
        }
        Self {
            jobs,
            done,
            pending: HashSet::new(),
        }
    }

    /// Queues `chunk` unless it is already on its way.
    pub fn request(&mut self, epoch: u64, planner: &Planner, chunk: i64) {
        if self.pending.insert((epoch, chunk)) {
            let _ = self.jobs.send(Job {
                epoch,
                planner: planner.clone(),
                chunk,
            });
        }
    }

    /// Finished chunks, without blocking.
    pub fn collect(&mut self) -> Vec<Done> {
        let out: Vec<Done> = self.done.try_iter().collect();
        for d in &out {
            self.pending.remove(&(d.epoch, d.chunk));
        }
        out
    }

    pub fn busy(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Forgets requests of older worlds (their results will be ignored).
    pub fn retire_before(&mut self, epoch: u64) {
        self.pending.retain(|(e, _)| *e >= epoch);
    }
}

fn work(jobs: &Mutex<Receiver<Job>>, done: &Sender<Done>) {
    loop {
        let job = match jobs.lock() {
            Ok(rx) => rx.recv(),
            Err(_) => return,
        };
        let Ok(job) = job else { return };
        let scenes = chunk_scenes(&job.planner, job.chunk);
        if done
            .send(Done {
                epoch: job.epoch,
                chunk: job.chunk,
                scenes,
            })
            .is_err()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;
    use std::time::{Duration, Instant};

    #[test]
    fn generates_requested_chunks_once() {
        let world = World::new("w");
        let mut w = ChunkWorkers::start();
        w.request(1, world.planner(), 0);
        w.request(1, world.planner(), 0);
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut got = Vec::new();
        while got.is_empty() && Instant::now() < deadline {
            got.extend(w.collect());
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].epoch, got[0].chunk), (1, 0));
        assert!(!w.busy());
    }
}

// use std::{ops::RangeInclusive, sync::{atomic::AtomicU64, Arc}};

// use bytes::Bytes;
// use range_set::RangeSet;
// use tokio::sync::Semaphore;

// use crate::{error::FilenSDKError, httpclient::FsURL};

// const MAX_CONCURRENT_THREADS: usize = 50;

// pub type TaskThreadFn<T> = Box<dyn Fn(FsURL, u64) -> T + Send + Sync + Clone + 'static>;

// pub struct TaskSpawnerCollector<T> {
//     pub(crate) task_spawner_tx: tokio::sync::mpsc::Sender<(u64, Option<T>)>,
//     pub(crate) task_spawner_rx: tokio::sync::mpsc::Receiver<(u64, Option<T>)>,
//     current_chunk: AtomicU64,
//     finished_chunks: RangeSet<[RangeInclusive<u64>; MAX_CONCURRENT_THREADS]>,
//     semaphore: Arc<Semaphore>,
// }

// pub trait TaskSpawner {
//     fn spawn_task(&self, url: FsURL, chunk: u64);
//     fn get_total_chunks(&self) -> u64;
//     fn get_uid(&self) -> String;
// }



// impl<T> TaskSpawnerCollector<T> {
//     pub fn new(semaphore: Arc<Semaphore>) -> Self {
//         let (task_spawner_tx, task_spawner_rx) = tokio::sync::mpsc::channel(100);

//         Self {
//             task_spawner_tx,
//             task_spawner_rx,
//             current_chunk: 0,
//             finished_chunks: RangeSet::new(),
//             semaphore,
//         }
//     }

//     pub fn start_spawning_tasks(&mut self) {
//     }
// }
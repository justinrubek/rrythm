use crate::{error::Result, task::Scheduled};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct Scheduler {
    tasks: Arc<RwLock<HashMap<String, Scheduled>>>,
    shutdown_tx: broadcast::Sender<()>,
    wakeup_tx: broadcast::Sender<()>,
}

impl Scheduler {
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let (wakeup_tx, _) = broadcast::channel(1);
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
            wakeup_tx,
        }
    }

    pub fn add_task(&self, task: Scheduled) {
        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(task.id.clone(), task);
        let _ = self.wakeup_tx.send(());
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    pub async fn run(&self) -> Result<()> {
        let task_count = self.tasks.read().unwrap().len();
        tracing::info!("scheduler starting with {} tasks", task_count);

        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let mut wakeup_rx = self.wakeup_tx.subscribe();

        loop {
            let mut soonest_time: Option<DateTime<Utc>> = None;

            {
                let tasks = self.tasks.read().unwrap();
                for task in tasks.values() {
                    if soonest_time.is_none() || task.next_run < soonest_time.unwrap() {
                        soonest_time = Some(task.next_run);
                    }
                }
            }

            let sleep_duration = match soonest_time {
                Some(next_time) => {
                    let now = Utc::now();
                    if next_time > now {
                        let duration = next_time - now;
                        let millis = duration.num_milliseconds().max(0);
                        if millis > 0 {
                            std::time::Duration::from_millis(millis.min(86_400_000) as u64)
                        } else {
                            std::time::Duration::from_millis(10)
                        }
                    } else {
                        std::time::Duration::from_millis(10)
                    }
                }
                None => std::time::Duration::from_secs(1),
            };

            tokio::select! {
                () = tokio::time::sleep(sleep_duration) => {
                    let now = Utc::now();
                    tracing::debug!("woke up at {}", now.format("%Y-%m-%d %H:%M:%S UTC"));

                    let mut tasks = self.tasks.write().unwrap();
                    let mut tasks_to_run = Vec::new();

                    for task in tasks.values() {
                        if task.should_run() {
                            tasks_to_run.push(task.id.clone());
                        }
                    }

                    for task_id in tasks_to_run {
                        if let Some(task) = tasks.get_mut(&task_id) {
                            tracing::info!(task_id, "executing task");
                            task.last_run = Some(Utc::now());

                            task.update_next_run();

                            tracing::info!(
                                task_id,
                                next_run = %task.next_run.format("%Y-%m-%d %H:%M:%S UTC"),
                                "next run"
                            );
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("scheduler shutting down");
                    return Ok(());
                }
                _ = wakeup_rx.recv() => {
                    tracing::debug!("woken early due to task change");
                }
            }
        }
    }
}

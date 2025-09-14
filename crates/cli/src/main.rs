use std::{str::FromStr, sync::Arc};

use crate::{commands::Commands, scheduler::Scheduler, task::Scheduled};
use clap::Parser;
use rrule::RRuleSet;

mod commands;
mod error;
mod scheduler;
mod task;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Registration {
    name: String,
    #[serde(rename = "rrule")]
    rrule_str: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ScheduleInfo {
    registrations: Vec<Registration>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct EventOccurance {
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = commands::Args::parse();
    match args.command {
        Commands::Server(server) => server.run().await?,
        Commands::Debug => {
            let file_contents = tokio::fs::read_to_string("registrations.ron").await?;
            let content: ScheduleInfo = ron::from_str(&file_contents)?;

            let scheduler = Scheduler::new();
            for r in content.registrations {
                let recurrence = RRuleSet::from_str(&r.rrule_str)?;

                let task = Scheduled::new(&r.name, recurrence).expect("failed to create task");
                scheduler.add_task(task);
            }

            let scheduler = Arc::new(scheduler);

            let scheduler_clone = Arc::clone(&scheduler);
            let scheduler_handle = tokio::spawn(async move {
                if let Err(e) = scheduler_clone.run().await {
                    tracing::error!("scheduler: {e}");
                }
            });

            tokio::signal::ctrl_c().await?;
            tracing::info!("received shutdown signal");
            scheduler.shutdown();
            scheduler_handle.await?;
        }
    }

    Ok(())
}

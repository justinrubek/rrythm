use crate::{commands::Commands, error::Error, scheduler::Scheduler, task::Scheduled};
use clap::Parser;
use rrule::RRuleSet;
use std::{str::FromStr, sync::Arc};
use tracing::info;

mod commands;
mod error;
mod scheduler;
mod task;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Registration {
    #[serde(default)]
    action: Action,
    name: String,
    #[serde(rename = "rrule")]
    rrule_str: String,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
enum Action {
    FetchAirgradient(FetchAirgradientConfig),
    #[default]
    Print,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct FetchAirgradientConfig {
    host: String,
}

pub struct FetchAirgradient {
    config: FetchAirgradientConfig,
    client: reqwest::Client,
}

impl FetchAirgradient {
    #[must_use]
    pub fn new(config: FetchAirgradientConfig, client: reqwest::Client) -> Self {
        Self { config, client }
    }
}

#[async_trait::async_trait]
impl scheduler::Action for FetchAirgradient {
    async fn execute(&self, context: &scheduler::ExecutionContext) -> Result<(), Error> {
        info!(host=?self.config.host, timestamp=?context.execution_time, "fetch airgradient");
        Ok(())
    }
}

pub struct Print {
    message: String,
}

#[async_trait::async_trait]
impl scheduler::Action for Print {
    async fn execute(&self, context: &scheduler::ExecutionContext) -> Result<(), Error> {
        info!(?self.message, timestamp=?context.execution_time);
        Ok(())
    }
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

            let http_client = reqwest::Client::new();

            let scheduler = Scheduler::new();
            for r in content.registrations {
                let name = r.name.clone();
                let recurrence = RRuleSet::from_str(&r.rrule_str)?;

                let action: Arc<dyn scheduler::Action> = match r.action {
                    Action::FetchAirgradient(fa) => {
                        Arc::new(FetchAirgradient::new(fa, http_client.clone()))
                    }
                    Action::Print => Arc::new(Print { message: r.name }),
                };

                let task =
                    Scheduled::new(&name, recurrence, action).expect("failed to create task");
                scheduler.add_task(task).await;
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

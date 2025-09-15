use crate::{
    error::{Error, Result},
    scheduler::{self, Action},
};
use chrono::{DateTime, Utc};
use rrule::{RRuleSet, RRuleSetIter};
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct Scheduled {
    pub id: String,
    pub recurrence: RRuleSet,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: DateTime<Utc>,
    pub action: Arc<dyn Action>,
    occurrence_iter: RRuleSetIter,
}

#[async_trait::async_trait]
impl scheduler::Action for Scheduled {
    async fn execute(&self, context: &scheduler::ExecutionContext) -> Result<()> {
        self.action.execute(context).await?;
        Ok(())
    }
}

impl Scheduled {
    pub fn new(id: &str, recurrence: RRuleSet, action: Arc<dyn Action>) -> Result<Self> {
        let now = Utc::now();

        let mut iter = recurrence.clone().into_iter();

        let mut next_run = None;
        for occurrence in iter.by_ref() {
            let occurence = occurrence.with_timezone(&Utc);
            if occurence >= now {
                next_run = Some(occurence);
                break;
            }
        }

        let next_run =
            next_run.ok_or_else(|| Error::Other("No future occurrences found".to_string()))?;

        tracing::debug!(
            id,
            now = %now.format("%Y-%m-%d %H:%M:%S UTC"),
            dt_start = %recurrence.get_dt_start().format("%Y-%m-%d %H:%M:%S UTC"),
            next_run = %next_run.format("%Y-%m-%d %H:%M:%S UTC"),
            "task initialized"
        );

        Ok(Self {
            id: id.to_string(),
            recurrence,
            last_run: None,
            next_run,
            action,
            occurrence_iter: iter,
        })
    }

    pub fn update_next_run(&mut self) {
        let now = Utc::now();

        if let Some(next_occurrence) = self.occurrence_iter.next() {
            self.next_run = next_occurrence.with_timezone(&Utc);
        } else {
            // TODO: this doesn't seem right, what should we do differently?

            // if the iterator is exhausted (unlikely for most recurrence rules)
            // create a new iterator and find the next occurrence >= now
            let mut new_iter = self.recurrence.clone().into_iter();
            while let Some(occurrence) = new_iter.next() {
                let occurrence_utc = occurrence.with_timezone(&Utc);
                if occurrence_utc >= now {
                    self.next_run = occurrence_utc;
                    self.occurrence_iter = new_iter;
                    return;
                }
            }

            tracing::error!("no future occurrences found for task {}", self.id);
            self.next_run = now + chrono::Duration::days(365);
        }

        tracing::debug!(
            id = %self.id,
            now = %now.format("%Y-%m-%d %H:%M:%S UTC"),
            next_run = %self.next_run.format("%Y-%m-%d %H:%M:%S UTC"),
            "Updated next run time"
        );
    }

    pub fn should_run(&self) -> bool {
        let now = Utc::now();
        let should_run = now >= self.next_run;

        if should_run {
            tracing::debug!(
                id = %self.id,
                now = %now.format("%Y-%m-%d %H:%M:%S UTC"),
                next_run = %self.next_run.format("%Y-%m-%d %H:%M:%S UTC"),
                "task should run"
            );
        }

        should_run
    }
}

impl fmt::Display for Scheduled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Task[{}] next run: {}",
            self.id,
            self.next_run.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

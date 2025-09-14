use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use rrule::{RRuleSet, Tz};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Scheduled {
    pub id: String,
    pub recurrence: RRuleSet,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: DateTime<Utc>,
}

impl Scheduled {
    pub fn new(id: &str, rrule_str: &str) -> Result<Self> {
        let now = Utc::now();

        let recurrence = RRuleSet::from_str(rrule_str)
            .map_err(|e| Error::Other(format!("Failed to parse RRULE: {}", e)))?;

        let next_run = Self::calculate_next_occurrence(&recurrence, now).unwrap();

        tracing::info!(
            id,
            now = %now.format("%Y-%m-%d %H:%M:%S UTC"),
            dt_start = %recurrence.get_dt_start().format("%Y-%m-%d %H:%M:%S UTC"),
            next_run = %next_run.format("%Y-%m-%d %H:%M:%S UTC"),
            "Task initialized"
        );

        Ok(Self {
            id: id.to_string(),
            recurrence,
            last_run: None,
            next_run,
        })
    }

    fn calculate_next_occurrence(
        recurrence: &RRuleSet,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        let now_rrule = now.with_timezone(&Tz::UTC);

        let occurence = recurrence.clone().after(now_rrule).all(1);
        let next_occurrence = occurence.dates.first();

        Some(next_occurrence?.with_timezone(&Utc))
    }

    pub fn update_next_run(&mut self) {
        let now = Utc::now();

        self.next_run = Self::calculate_next_occurrence(&self.recurrence, now)
            .expect("Failed to calculate next run time");

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

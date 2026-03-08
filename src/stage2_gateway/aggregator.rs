use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor_id: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBatch {
    pub batch_id: String,
    pub readings: Vec<SensorReading>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
}

pub struct BatchAggregator {
    pub window_duration_secs: i64,
    pub current_batch: Vec<SensorReading>,
    pub last_window_end: DateTime<Utc>,
}

impl BatchAggregator {
    pub fn new(window_duration_secs: i64) -> Self {
        Self {
            window_duration_secs,
            current_batch: Vec::new(),
            last_window_end: Utc::now(),
        }
    }

    pub fn add_reading(&mut self, reading: SensorReading) {
        self.current_batch.push(reading);
    }

    pub fn should_seal(&self) -> bool {
        let now = Utc::now();
        (now - self.last_window_end).num_seconds() >= self.window_duration_secs
    }

    pub fn seal_batch(&mut self) -> Option<DataBatch> {
        if self.current_batch.is_empty() {
            return None;
        }

        let now = Utc::now();
        let batch = DataBatch {
            batch_id: uuid::Uuid::new_v4().to_string(),
            readings: self.current_batch.drain(..).collect(),
            window_start: self.last_window_end,
            window_end: now,
        };

        self.last_window_end = now;
        Some(batch)
    }
}

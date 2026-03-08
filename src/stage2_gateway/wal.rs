use std::fs::{OpenOptions, File};
use std::io::{Write, BufRead, BufReader};
use crate::stage2_gateway::aggregator::DataBatch;

pub struct WriteAheadLog {
    log_path: String,
}

impl WriteAheadLog {
    pub fn new(log_path: &str) -> Self {
        Self { log_path: log_path.to_string() }
    }

    pub fn write_batch(&self, batch: &DataBatch) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        let serialized = serde_json::to_string(batch).unwrap();
        writeln!(file, "{}", serialized)?;
        file.sync_all() // Ensure ACID durability
    }

    pub fn recover(&self) -> std::io::Result<Vec<DataBatch>> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut batches = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if let Ok(batch) = serde_json::from_str::<DataBatch>(&line) {
                batches.push(batch);
            }
        }

        Ok(batches)
    }

    pub fn clear(&self) -> std::io::Result<()> {
        std::fs::remove_file(&self.log_path)
    }
}

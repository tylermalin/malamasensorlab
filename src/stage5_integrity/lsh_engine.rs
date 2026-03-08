use sha2::{Sha256, Digest};

pub struct LshEngine {
    pub seed: u64,
}

impl LshEngine {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Generates a 64-bit 'SimHash' fingerprint for a data batch.
    /// This allows us to calculate Hamming Distance for anomaly detection.
    pub fn fingerprint(&self, data: &[u8]) -> u64 {
        let mut v = vec![0i32; 64];
        
        // In a real LSH, we'd tokenize data. Here we use sliding windows of 8 bytes.
        for chunk in data.chunks(8) {
            let mut hasher = Sha256::new();
            hasher.update(self.seed.to_be_bytes());
            hasher.update(chunk);
            let hash = hasher.finalize();
            
            for i in 0..64 {
                let bit = (hash[i / 8] >> (i % 8)) & 1;
                if bit == 1 {
                    v[i] += 1;
                } else {
                    v[i] -= 1;
                }
            }
        }

        let mut fingerprint = 0u64;
        for i in 0..64 {
            if v[i] > 0 {
                fingerprint |= 1 << i;
            }
        }
        fingerprint
    }

    /// Calculates Hamming Distance between two fingerprints.
    pub fn hamming_distance(a: u64, b: u64) -> u32 {
        (a ^ b).count_ones()
    }

    /// Prompt 33: Specialized LSH Fingerprint for sensor readings.
    /// Summarizes data into a 32-byte hash based on statistical properties.
    pub fn compute_lsh_fingerprint(readings: &[f64]) -> [u8; 32] {
        if readings.is_empty() {
            return [0u8; 32];
        }
        let len = readings.len() as f64;
        let mean = readings.iter().sum::<f64>() / len;
        let variance = readings.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / len;
        let std_dev = variance.sqrt();
        let min = readings.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = readings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Format: "mean|std_dev|min|max" rounded to 2 decimal places 
        // to provide a stable statistical fingerprint.
        let fingerprint_str = format!("{:.2}|{:.2}|{:.2}|{:.2}", mean, std_dev, min, max);
        
        let mut hasher = Sha256::new();
        hasher.update(fingerprint_str.as_bytes());
        let result = hasher.finalize();
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

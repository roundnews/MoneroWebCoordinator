use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use num_bigint::BigUint;
use once_cell::sync::Lazy;

use crate::template::TemplateState;

// Pre-compute 2^256 once for efficiency
static MAX_TARGET: Lazy<BigUint> = Lazy::new(|| {
    let two: BigUint = 2u32.into();
    two.pow(256)
});

#[derive(Clone, Debug)]
pub struct Job {
    pub job_id: String,
    pub template_id: u64,
    pub blob_hex: String,
    pub reserved_offset: usize,
    pub reserved_value: Vec<u8>,
    pub target_hex: String,
    pub height: u64,
    pub seed_hash: String,
    pub created_at: Instant,
}

pub struct JobManager {
    jobs: DashMap<String, Job>,
    counter: AtomicU64,
    stale_grace_ms: u64,
}

impl JobManager {
    pub fn new(stale_grace_ms: u64) -> Self {
        Self {
            jobs: DashMap::new(),
            counter: AtomicU64::new(0),
            stale_grace_ms,
        }
    }

    pub fn create_job(&self, template: &TemplateState, session_id: &str) -> Job {
        let seq = self.counter.fetch_add(1, Ordering::SeqCst);
        let job_id = format!("{:016x}", seq);
        
        // Create unique reserved value from session + sequence
        let mut reserved = vec![0u8; template.reserve_size as usize];
        let session_bytes = session_id.as_bytes();
        let seq_bytes = seq.to_le_bytes();
        
        for (i, byte) in session_bytes.iter().chain(seq_bytes.iter()).take(reserved.len()).enumerate() {
            reserved[i] = *byte;
        }

        // Modify blob with reserved value
        let mut blob = hex::decode(&template.blocktemplate_blob).unwrap_or_default();
        let offset = template.reserved_offset;
        for (i, byte) in reserved.iter().enumerate() {
            if offset + i < blob.len() {
                blob[offset + i] = *byte;
            }
        }

        // Calculate target from difficulty
        let target = difficulty_to_target(template.difficulty);

        let job = Job {
            job_id: job_id.clone(),
            template_id: template.template_id,
            blob_hex: hex::encode(&blob),
            reserved_offset: offset,
            reserved_value: reserved,
            target_hex: hex::encode(&target),
            height: template.height,
            seed_hash: template.seed_hash.clone(),
            created_at: Instant::now(),
        };

        self.jobs.insert(job_id, job.clone());
        job
    }

    pub fn get_job(&self, job_id: &str) -> Option<Job> {
        self.jobs.get(job_id).map(|j| j.clone())
    }

    pub fn is_stale(&self, job: &Job, current_template_id: u64) -> bool {
        if job.template_id == current_template_id {
            return false;
        }
        job.created_at.elapsed().as_millis() > self.stale_grace_ms as u128
    }

    pub fn cleanup_old_jobs(&self, max_age_ms: u64) {
        self.jobs.retain(|_, job| {
            job.created_at.elapsed().as_millis() < max_age_ms as u128
        });
    }
}

fn difficulty_to_target(difficulty: u64) -> [u8; 32] {
    if difficulty <= 1 {
        return [0xff; 32];
    }

    // Target = 2^256 / difficulty
    let diff_big: BigUint = difficulty.into();
    let target_big = &*MAX_TARGET / &diff_big;

    // Convert to 32-byte little-endian array
    let target_bytes = target_big.to_bytes_le();
    
    let mut target = [0u8; 32];
    let len = target_bytes.len().min(32);
    target[..len].copy_from_slice(&target_bytes[..len]);

    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_to_target_low() {
        // For difficulty 1, should return max target
        let target = difficulty_to_target(1);
        assert_eq!(target, [0xff; 32]);
    }

    #[test]
    fn test_difficulty_to_target_basic() {
        // For difficulty 2, target should be 2^255 (half of max)
        let target = difficulty_to_target(2);
        
        // Verify it's a valid 32-byte array with non-zero values
        assert_ne!(target, [0u8; 32]);
        
        // For difficulty 2, result is 2^255
        // In little-endian: bytes[0..30] = 0x00, byte[31] = 0x80
        assert_eq!(target[31], 0x80); // MSB should be 0x80 for 2^255
        assert_eq!(target[30], 0x00);
    }

    #[test]
    fn test_difficulty_to_target_high() {
        // For high difficulty, target should be small
        let target = difficulty_to_target(1_000_000);
        
        // Should have non-zero bytes in lower positions
        let has_nonzero = target.iter().any(|&b| b != 0);
        assert!(has_nonzero, "Target should have at least some non-zero bytes");
        
        // High bytes should be zero for high difficulty
        assert_eq!(target[31], 0);
        assert_eq!(target[30], 0);
    }

    #[test]
    fn test_difficulty_to_target_produces_32_bytes() {
        // Verify all difficulties produce 32-byte targets
        for difficulty in [1, 2, 10, 100, 1000, 10000, 100000, 1_000_000].iter() {
            let target = difficulty_to_target(*difficulty);
            assert_eq!(target.len(), 32);
        }
    }
}

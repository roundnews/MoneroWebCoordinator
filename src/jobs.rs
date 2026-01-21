use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::template::TemplateState;

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
    // We compute this by dividing the maximum 128-bit value by difficulty
    // and placing the result in the upper 16 bytes of the 32-byte target array
    // (in little-endian format, bytes 16-31)
    
    let mut target = [0u8; 32];
    
    // For difficulties that fit in u64, use simplified calculation
    // Compute high 128 bits of (2^256-1) / difficulty approximation
    let target_value: u128 = u128::MAX / difficulty as u128;
    let target_bytes = target_value.to_le_bytes();
    
    // Place in upper portion of target (little-endian, so bytes 16-31)
    target[16..32].copy_from_slice(&target_bytes);
    
    target
}

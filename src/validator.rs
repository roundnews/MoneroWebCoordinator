use randomx_rs::{RandomXCache, RandomXFlag, RandomXVM};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::jobs::Job;
use crate::error::CoordinatorError;

pub struct SubmissionValidator {
    min_blob_len: usize,
    vm: Arc<RwLock<Option<RandomXVM>>>,
    current_seed_hash: Arc<RwLock<String>>,
}

// Safety: RandomXVM is protected by RwLock, so concurrent access is properly synchronized.
// The RwLock ensures that only one thread can mutate at a time, and multiple threads can read safely.
unsafe impl Send for SubmissionValidator {}
unsafe impl Sync for SubmissionValidator {}

impl SubmissionValidator {
    pub fn new() -> Self {
        Self {
            min_blob_len: 76,
            vm: Arc::new(RwLock::new(None)),
            current_seed_hash: Arc::new(RwLock::new(String::new())),
        }
    }

    /// Initialize or reinitialize the RandomX VM with a new seed hash
    pub fn init_vm(&self, seed_hash: &str) -> Result<(), CoordinatorError> {
        let mut current = self.current_seed_hash.write();
        if *current == seed_hash {
            return Ok(()); // Already initialized with this seed
        }

        let seed_bytes = hex::decode(seed_hash)
            .map_err(|_| CoordinatorError::Validation("Invalid seed hash hex".into()))?;

        let flags = RandomXFlag::get_recommended_flags();
        let cache = RandomXCache::new(flags, &seed_bytes)
            .map_err(|e| CoordinatorError::Validation(format!("RandomX cache init failed: {}", e)))?;
        
        let vm = RandomXVM::new(flags, Some(cache), None)
            .map_err(|e| CoordinatorError::Validation(format!("RandomX VM init failed: {}", e)))?;

        let mut vm_lock = self.vm.write();
        *vm_lock = Some(vm);
        *current = seed_hash.to_string();

        tracing::info!("RandomX VM initialized with seed: {}", seed_hash);
        Ok(())
    }

    pub fn validate_blob(&self, blob_hex: &str, job: &Job) -> Result<Vec<u8>, CoordinatorError> {
        let blob = hex::decode(blob_hex)
            .map_err(|_| CoordinatorError::Validation("Invalid hex".into()))?;

        if blob.len() < self.min_blob_len {
            return Err(CoordinatorError::Validation("Blob too short".into()));
        }

        let offset = job.reserved_offset;
        let reserved = &job.reserved_value;
        
        if offset + reserved.len() > blob.len() {
            return Err(CoordinatorError::Validation("Invalid blob structure".into()));
        }

        for (i, expected) in reserved.iter().enumerate() {
            if blob[offset + i] != *expected {
                return Err(CoordinatorError::Validation("Reserved value mismatch".into()));
            }
        }

        Ok(blob)
    }

    /// Compute RandomX hash of the blob
    pub fn compute_hash(&self, blob: &[u8]) -> Result<[u8; 32], CoordinatorError> {
        let vm_lock = self.vm.read();
        let vm = vm_lock.as_ref()
            .ok_or_else(|| CoordinatorError::Validation("RandomX VM not initialized".into()))?;

        let hash = vm.calculate_hash(blob)
            .map_err(|e| CoordinatorError::Validation(format!("Hash computation failed: {}", e)))?;

        if hash.len() != 32 {
            return Err(CoordinatorError::Validation(
                format!("Unexpected hash length: expected 32, got {}", hash.len())
            ));
        }

        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        Ok(result)
    }

    pub fn check_meets_target(&self, hash: &[u8; 32], target: &[u8; 32]) -> bool {
        for i in (0..32).rev() {
            if hash[i] < target[i] {
                return true;
            }
            if hash[i] > target[i] {
                return false;
            }
        }
        true
    }
}

use sha3::{Digest, Keccak256};

use crate::jobs::Job;
use crate::error::CoordinatorError;

pub struct SubmissionValidator {
    min_blob_len: usize,
}

impl SubmissionValidator {
    pub fn new() -> Self {
        Self {
            min_blob_len: 76, // Minimum valid blob size
        }
    }

    pub fn validate_blob(&self, blob_hex: &str, job: &Job) -> Result<Vec<u8>, CoordinatorError> {
        let blob = hex::decode(blob_hex)
            .map_err(|_| CoordinatorError::Validation("Invalid hex".into()))?;

        if blob.len() < self.min_blob_len {
            return Err(CoordinatorError::Validation("Blob too short".into()));
        }

        // Verify reserved region matches assigned value
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

    /// Compute hash of the blob (simplified - real implementation needs RandomX)
    /// For now, use Keccak256 as placeholder until RandomX integration
    pub fn compute_hash(&self, blob: &[u8]) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        hasher.update(blob);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Check if hash meets the target difficulty
    pub fn check_meets_target(&self, hash: &[u8; 32], target: &[u8; 32]) -> bool {
        // Compare hash against target (both little-endian)
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

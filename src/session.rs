use dashmap::DashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::ratelimit::SessionLimits;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Connected,
    Ready,
    Closed,
}

pub struct Session {
    pub id: String,
    pub ip: IpAddr,
    pub state: SessionState,
    pub client_version: Option<String>,
    pub threads: u8,
    pub current_job_id: Option<String>,
    pub current_reserved_value: Option<Vec<u8>>,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub limits: SessionLimits,
}

impl Session {
    pub fn new(ip: IpAddr, messages_per_second: u32, submits_per_minute: u32) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            ip,
            state: SessionState::Connected,
            client_version: None,
            threads: 1,
            current_job_id: None,
            current_reserved_value: None,
            connected_at: now,
            last_activity: now,
            limits: SessionLimits::new(messages_per_second, submits_per_minute),
        }
    }

    pub fn set_ready(&mut self, client_version: String, threads: u8) {
        self.client_version = Some(client_version);
        self.threads = threads;
        self.state = SessionState::Ready;
    }

    pub fn update_job(&mut self, job_id: String, reserved_value: Vec<u8>) {
        self.current_job_id = Some(job_id);
        self.current_reserved_value = Some(reserved_value);
        self.last_activity = Instant::now();
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn check_message_limit(&mut self) -> bool {
        self.limits.messages.check()
    }

    pub fn check_submit_limit(&mut self) -> bool {
        self.limits.submits.check()
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            ip: self.ip,
            state: self.state,
            client_version: self.client_version.clone(),
            threads: self.threads,
            current_job_id: self.current_job_id.clone(),
            current_reserved_value: self.current_reserved_value.clone(),
            connected_at: self.connected_at,
            last_activity: self.last_activity,
            limits: SessionLimits::new(20, 120), // Default values for clones
        }
    }
}

pub struct SessionManager {
    sessions: DashMap<String, Session>,
    ip_counts: DashMap<IpAddr, usize>,
    max_per_ip: usize,
    max_total: usize,
    messages_per_second: u32,
    submits_per_minute: u32,
}

impl SessionManager {
    pub fn new(max_per_ip: usize, max_total: usize, messages_per_second: u32, submits_per_minute: u32) -> Self {
        Self {
            sessions: DashMap::new(),
            ip_counts: DashMap::new(),
            max_per_ip,
            max_total,
            messages_per_second,
            submits_per_minute,
        }
    }

    pub fn create_session(&self, ip: IpAddr) -> Option<Session> {
        // Check global limit FIRST
        if self.sessions.len() >= self.max_total {
            return None;
        }
        
        // Then check per-IP limit
        let mut count = self.ip_counts.entry(ip).or_insert(0);
        if *count >= self.max_per_ip {
            return None;
        }
        *count += 1;
        
        let session = Session::new(ip, self.messages_per_second, self.submits_per_minute);
        self.sessions.insert(session.id.clone(), session.clone());
        Some(session)
    }

    pub fn get_session(&self, id: &str) -> Option<Session> {
        self.sessions.get(id).map(|s| s.clone())
    }

    pub fn update_session<F>(&self, id: &str, f: F)
    where
        F: FnOnce(&mut Session),
    {
        if let Some(mut session) = self.sessions.get_mut(id) {
            f(&mut session);
        }
    }

    pub fn check_message_limit(&self, id: &str) -> bool {
        if let Some(mut session) = self.sessions.get_mut(id) {
            return session.check_message_limit();
        }
        false
    }

    pub fn check_submit_limit(&self, id: &str) -> bool {
        if let Some(mut session) = self.sessions.get_mut(id) {
            return session.check_submit_limit();
        }
        false
    }

    pub fn remove_session(&self, id: &str) {
        if let Some((_, session)) = self.sessions.remove(id) {
            let mut count = self.ip_counts.entry(session.ip).or_insert(0);
            *count = count.saturating_sub(1);
            if *count == 0 {
                drop(count);
                self.ip_counts.remove(&session.ip);
            }
        }
    }

    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    /// Remove sessions that have been idle for longer than the specified duration
    pub fn cleanup_idle(&self, max_idle: Duration) -> usize {
        let now = Instant::now();
        let mut removed = 0;
        
        // Collect IDs to remove (can't modify while iterating)
        let to_remove: Vec<String> = self.sessions
            .iter()
            .filter(|entry| now.duration_since(entry.value().last_activity) > max_idle)
            .map(|entry| entry.key().clone())
            .collect();
        
        for id in to_remove {
            self.remove_session(&id);
            removed += 1;
        }
        
        if removed > 0 {
            tracing::info!("Cleaned up {} idle sessions", removed);
        }
        
        removed
    }
}

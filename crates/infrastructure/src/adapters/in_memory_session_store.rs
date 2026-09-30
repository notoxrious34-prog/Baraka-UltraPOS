//! Volatile in-memory session store (never persisted to disk).
//!
//! Sessions live behind a `Mutex<HashMap>` with fixed TTL expiry enforced
//! on every validation; expired entries are evicted on sight.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use baraka_application::ports::{Clock, SessionContext, SessionStore, SessionToken};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use chrono::{DateTime, TimeDelta, Utc};

struct StoredSession {
    user_id: UserId,
    role: Role,
    expires_at: DateTime<Utc>,
}

/// Thread-safe volatile session registry.
pub struct InMemorySessionStore {
    sessions: Mutex<HashMap<String, StoredSession>>,
    clock: Arc<dyn Clock>,
}

impl InMemorySessionStore {
    /// Build an empty store around the given clock.
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            clock,
        }
    }
}

impl SessionStore for InMemorySessionStore {
    fn create_session(&self, user_id: UserId, role: Role, ttl_seconds: i64) -> SessionToken {
        let token = SessionToken::new_random();
        let stored = StoredSession {
            user_id,
            role,
            expires_at: self.clock.now() + TimeDelta::seconds(ttl_seconds),
        };
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(token.0.clone(), stored);
        }
        token
    }

    fn validate_session(&self, token: &SessionToken) -> Option<SessionContext> {
        let mut sessions = self.sessions.lock().ok()?;
        match sessions.get(&token.0) {
            Some(stored) if stored.expires_at > self.clock.now() => Some(SessionContext {
                token: token.clone(),
                user_id: stored.user_id,
                role: stored.role,
                expires_at: stored.expires_at,
            }),
            _ => {
                sessions.remove(&token.0);
                None
            }
        }
    }

    fn invalidate_session(&self, token: &SessionToken) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&token.0);
        }
    }
}

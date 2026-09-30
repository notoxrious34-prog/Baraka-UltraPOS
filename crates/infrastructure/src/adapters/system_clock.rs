//! Canonical system clock implementing the application `Clock` port.

use baraka_application::ports::Clock;
use chrono::{DateTime, SecondsFormat, Utc};

/// Production clock (OS time via chrono, no I/O).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn now_rfc3339(&self) -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

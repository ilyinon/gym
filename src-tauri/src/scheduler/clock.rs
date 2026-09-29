use std::time::{SystemTime, UNIX_EPOCH};

pub trait Clock: Send + Sync {
    fn now_utc_ms(&self) -> i64;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after Unix epoch")
            .as_millis() as i64
    }
}

#[cfg(test)]
pub struct FakeClock {
    now_utc_ms: std::sync::atomic::AtomicI64,
}

#[cfg(test)]
impl FakeClock {
    pub fn new(now_utc_ms: i64) -> Self {
        Self {
            now_utc_ms: std::sync::atomic::AtomicI64::new(now_utc_ms),
        }
    }

    pub fn set(&self, now_utc_ms: i64) {
        self.now_utc_ms
            .store(now_utc_ms, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
impl Clock for FakeClock {
    fn now_utc_ms(&self) -> i64 {
        self.now_utc_ms.load(std::sync::atomic::Ordering::SeqCst)
    }
}

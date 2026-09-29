mod clock;
mod engine;
mod schedule;

pub use engine::{start_scheduler, SchedulerHandle};
pub(crate) use schedule::next_due_from_utc;

pub(crate) fn current_time_utc_ms() -> i64 {
    use clock::{Clock, SystemClock};

    SystemClock.now_utc_ms()
}

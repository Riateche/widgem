use {
    crate::callback::Callback,
    priority_queue::PriorityQueue,
    std::{
        cmp::Reverse,
        collections::HashMap,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    },
};

#[derive(Default)]
pub struct Timers {
    queue: PriorityQueue<TimerId, Reverse<Instant>>,
    timers: HashMap<TimerId, Timer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(pub u64);

impl TimerId {
    pub(crate) fn new_unique() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone)]
pub struct Timer {
    pub interval: Option<Duration>,
    pub callback: Callback<Instant>,
}

impl Timers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, instant: Instant, timer: Timer) -> TimerId {
        let id = TimerId::new_unique();
        self.add_with_id(instant, timer, id);
        id
    }

    fn add_with_id(&mut self, instant: Instant, timer: Timer, id: TimerId) {
        self.queue.push(id, Reverse(instant));
        self.timers.insert(id, timer);
    }

    pub fn remove(&mut self, id: TimerId) {
        self.queue.remove(&id);
        self.timers.remove(&id);
    }

    pub fn next_instant(&self) -> Option<Instant> {
        self.queue.peek().map(|(_item, instant)| instant.0)
    }

    pub fn next_ready_timer(&mut self) -> Option<Timer> {
        let next = self.next_instant()?;
        if next > Instant::now() {
            return None;
        }
        let (id, old_instant) = self.queue.pop().unwrap();
        let timer = self.timers.remove(&id).expect("missing entry in timers");
        if let Some(interval) = timer.interval {
            self.add_with_id(old_instant.0 + interval, timer.clone(), id);
        }
        Some(timer)
    }
}

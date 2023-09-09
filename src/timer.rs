use std::{
    cmp::Reverse,
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use priority_queue::PriorityQueue;

use crate::{callback::WidgetCallback, system::with_system};

#[derive(Default)]
pub struct Timers {
    queue: PriorityQueue<TimerId, Reverse<Instant>>,
    timers: HashMap<TimerId, WidgetTimer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(pub u64);

impl TimerId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn cancel(self) {
        with_system(|system| system.timers.remove(self))
    }
}

#[derive(Clone)]
pub struct WidgetTimer {
    pub interval: Option<Duration>,
    pub callback: WidgetCallback<Instant>,
}

impl Timers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, instant: Instant, timer: WidgetTimer) -> TimerId {
        let id = TimerId::new();
        self.add_with_id(instant, timer, id);
        id
    }

    fn add_with_id(&mut self, instant: Instant, timer: WidgetTimer, id: TimerId) {
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

    pub fn pop(&mut self) -> Option<WidgetTimer> {
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

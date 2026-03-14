use super::klein_event::{KleinEvent, TimerKind};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Manages debounce timers that fire KleinEvent::Timer events.
///
/// Rescheduling a timer cancels the previous one — only the latest fires.
pub struct TimerManager {
    event_tx: mpsc::UnboundedSender<KleinEvent>,
    active: HashMap<TimerKind, JoinHandle<()>>,
}

impl TimerManager {
    pub fn new(event_tx: mpsc::UnboundedSender<KleinEvent>) -> Self {
        Self {
            event_tx,
            active: HashMap::new(),
        }
    }

    /// Schedule (or reschedule) a debounce timer.
    /// If a timer of this kind is already running, it is cancelled first.
    pub fn schedule(&mut self, kind: TimerKind, delay: Duration) {
        if let Some(handle) = self.active.remove(&kind) {
            handle.abort();
        }

        let tx = self.event_tx.clone();
        let k = kind.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = tx.send(KleinEvent::Timer(k));
        });

        self.active.insert(kind, handle);
    }

    /// Cancel a running timer without firing it.
    #[allow(dead_code)]
    pub fn cancel(&mut self, kind: &TimerKind) {
        if let Some(handle) = self.active.remove(kind) {
            handle.abort();
        }
    }

    /// Cancel all running timers.
    #[allow(dead_code)]
    pub fn cancel_all(&mut self) {
        for (_, handle) in self.active.drain() {
            handle.abort();
        }
    }
}

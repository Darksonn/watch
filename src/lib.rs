//! This crate provides a synchronous message passing channel that only
//! retains the most recent value.
//!
//! This crate provides a `parking_lot` feature. When enabled, the crate will
//! use the mutex from the `parking_lot` crate rather than the one from std.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(not(feature = "parking_lot"))]
mod sync_std;
#[cfg(not(feature = "parking_lot"))]
use sync_std::{Condvar, Mutex};

#[cfg(feature = "parking_lot")]
mod sync_parking_lot;
#[cfg(feature = "parking_lot")]
use sync_parking_lot::{Condvar, Mutex};

/// The sender for the watch channel.
///
/// The sender can be cloned to obtain multiple senders for the same channel.
pub struct WatchSender<T> {
    shared: Arc<Shared<T>>,
}

/// The receiver for the watch channel.
///
/// The receiver can be cloned. Each clone will yield a new receiver that
/// receives the same messages.
pub struct WatchReceiver<T> {
    shared: Arc<Shared<T>>,
    last_seen_version: u64,
}

impl<T> Clone for WatchSender<T> {
    fn clone(&self) -> WatchSender<T> {
        WatchSender {
            shared: self.shared.clone(),
        }
    }
}
impl<T> Clone for WatchReceiver<T> {
    fn clone(&self) -> WatchReceiver<T> {
        WatchReceiver {
            shared: self.shared.clone(),
            last_seen_version: self.last_seen_version,
        }
    }
}

struct Shared<T> {
    lock: Mutex<SharedValue<T>>,
    on_update: Condvar,
}
struct SharedValue<T> {
    value: T,
    version: u64,
}

/// Creates a new watch channel.
///
/// The starting value in the channel is not initially considered seen by the receiver.
pub fn channel<T: Clone>(value: T) -> (WatchSender<T>, WatchReceiver<T>) {
    let shared = Arc::new(Shared {
        lock: Mutex::new(SharedValue { value, version: 1 }),
        on_update: Condvar::new(),
    });
    (
        WatchSender {
            shared: shared.clone(),
        },
        WatchReceiver {
            shared,
            last_seen_version: 0,
        },
    )
}

impl<T> WatchSender<T> {
    /// Send a new message and notify all receivers currently waiting for a
    /// message.
    pub fn send(&self, mut value: T) {
        {
            let mut lock = self.shared.lock.lock();
            std::mem::swap(&mut lock.value, &mut value);
            lock.version = lock.version.wrapping_add(1);
        }
        self.shared.on_update.notify_all();

        // Destroy old value after releasing lock.
        drop(value);
    }

    /// Update the message by a closure and notify all receivers currently waiting for a message.
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        {
            let mut lock = self.shared.lock.lock();
            f(&mut lock.value);
            lock.version = lock.version.wrapping_add(1);
        }
        self.shared.on_update.notify_all();
    }

    /// Create a new receiver for the channel.
    ///
    /// Any messages sent before this method was called are considered seen by
    /// the new receiver.
    pub fn subscribe(&self) -> WatchReceiver<T> {
        let version = {
            let lock = self.shared.lock.lock();
            lock.version
        };

        WatchReceiver {
            shared: self.shared.clone(),
            last_seen_version: version,
        }
    }
}

impl<T: Clone> WatchReceiver<T> {
    /// Get a clone of the latest value sent on the channel.
    pub fn get(&mut self) -> T {
        let lock = self.shared.lock.lock();
        self.last_seen_version = lock.version;
        lock.value.clone()
    }

    /// Get a clone of the latest value if that value has not previously been
    /// seen by this receiver.
    pub fn get_if_new(&mut self) -> Option<T> {
        let lock = self.shared.lock.lock();
        if self.last_seen_version == lock.version {
            return None;
        }
        self.last_seen_version = lock.version;
        Some(lock.value.clone())
    }

    /// This method waits until a new value becomes available and return a clone
    /// of it.
    pub fn wait(&mut self) -> T {
        let mut lock = self.shared.lock.lock();

        while lock.version == self.last_seen_version {
            lock = self.shared.on_update.wait(lock);
        }

        self.last_seen_version = lock.version;
        lock.value.clone()
    }

    /// This method waits until a new value becomes available and return a clone
    /// of it, timing out after specified duration.
    pub fn wait_timeout(&mut self, duration: Duration) -> Option<T> {
        let mut lock = self.shared.lock.lock();

        let deadline = Instant::now() + duration;

        while lock.version == self.last_seen_version {
            let timeout = deadline.saturating_duration_since(Instant::now());

            lock = self.shared.on_update.wait_timeout(lock, timeout)?;

            // Note: checking after `on_update.wait_timeout` to call it at least once,
            // event when `duration` was zero.
            if timeout.is_zero() && lock.version == self.last_seen_version {
                return None;
            }
        }

        self.last_seen_version = lock.version;
        Some(lock.value.clone())
    }
}

impl<T> WatchReceiver<T> {
    /// Create a new sender for this channel.
    pub fn new_sender(&self) -> WatchSender<T> {
        WatchSender {
            shared: self.shared.clone(),
        }
    }
}

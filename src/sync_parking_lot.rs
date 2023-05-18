use parking_lot::MutexGuard;
use std::time::Duration;

pub struct Mutex<T> {
    inner: parking_lot::Mutex<T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: parking_lot::Mutex::new(value),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.inner.lock()
    }
}

pub struct Condvar {
    inner: parking_lot::Condvar,
}
impl Condvar {
    pub fn new() -> Self {
        Self {
            inner: parking_lot::Condvar::new(),
        }
    }

    pub fn wait<'a, T>(&self, mut guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        self.inner.wait(&mut guard);
        guard
    }

    pub fn wait_timeout<'a, T>(
        &self,
        mut guard: MutexGuard<'a, T>,
        duration: Duration,
    ) -> Option<MutexGuard<'a, T>> {
        if self.inner.wait_for(&mut guard, duration).timed_out() {
            None
        } else {
            Some(guard)
        }
    }

    pub fn notify_all(&self) {
        self.inner.notify_all();
    }
}

use std::sync::MutexGuard;

pub struct Mutex<T> {
    inner: std::sync::Mutex<T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: std::sync::Mutex::new(value),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
    }
}

pub struct Condvar {
    inner: std::sync::Condvar,
}
impl Condvar {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Condvar::new(),
        }
    }

    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        match self.inner.wait(guard) {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
    }

    pub fn notify_all(&self) {
        self.inner.notify_all();
    }
}

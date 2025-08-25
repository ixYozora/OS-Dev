use core::arch::asm;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicBool;

/// A simple spinlock implementation that spins in a loop until it acquires the lock.
pub struct Spinlock<T> {
    /// The lock is represented by an atomic boolean that indicates whether the lock is held.
    lock: AtomicBool,
    /// The data protected by the spinlock, stored in an `UnsafeCell` to allow mutable access.
    /// We need to use `UnsafeCell` because we want to allow mutable access to the data from
    /// a const reference in `MutexGuard`. This effectively bypasses Rust's borrowing rules,
    /// but the `Spinlock` itself ensures that only one thread can access the data at a time.
    data: UnsafeCell<T>,
}

unsafe impl<T> Sync for Spinlock<T> where T: Send {}
unsafe impl<T> Send for Spinlock<T> where T: Send {}

impl<T> Spinlock<T> {
    /// Create a new `Spinlock` protecting the given data.
    pub const fn new(data: T) -> Self {
        Spinlock {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data)
        }
    }

    /// Try to acquire the lock once without blocking.
    pub fn try_lock(&self) -> Option<SpinlockGuard<T>> {

        let before = self.lock.swap(true, core::sync::atomic::Ordering::Acquire);

        if before {
            // lock was held
            return None;
        }

        Some(SpinlockGuard { lock: self })
    }

    /// Spin until the lock is acquired, then return a guard that allows access to the data.
    pub fn lock(&self) -> SpinlockGuard<T> {

        let mut before = self.lock.swap(true, core::sync::atomic::Ordering::Acquire);

        while before {

            unsafe {
                asm!("pause");
            }

            // new try, busypolling
            before = self.lock.swap(true, core::sync::atomic::Ordering::Acquire);
        }

        SpinlockGuard { lock: self }
    }

    /// Check if the lock is currently held.
    pub fn is_locked(&self) -> bool {

        self.lock.load(core::sync::atomic::Ordering::Relaxed)

    }

    /// Unlock the spinlock, allowing other threads to acquire it.
    pub fn unlock(&self) {

        if !self.is_locked() {
            panic!("Spinlock is not locked, cannot unlock");
        }

        self.lock.store(false, core::sync::atomic::Ordering::Release);

    }

    /// Forcefully unlock the spinlock. This should only be used in exceptional cases.
    pub unsafe fn force_unlock(&self) {

        //TODO: unsicher ob das richtig ist
        self.lock.store(false, core::sync::atomic::Ordering::Release);

    }
}

/// A guard that provides access to the data protected by the spinlock.
/// It implements `Deref` and `DerefMut` to allow transparent access to the data.
/// It also implements `Drop` to automatically unlock the spinlock when it goes out of scope.
pub struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>
}

impl<'a, T> Deref for SpinlockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            self.lock.data.get().as_ref().unwrap()
        }
    }
}

impl<'a, T> DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            self.lock.data.get().as_mut().unwrap()
        }
    }
}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
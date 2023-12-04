use std::sync::{
    atomic::{self, AtomicBool},
    LockResult, Mutex, MutexGuard,
};

pub struct RecessiveMutex<T> {
    wants_access: AtomicBool,
    mutex: Mutex<T>,
}

impl<T> RecessiveMutex<T> {
    pub fn new(t: T) -> RecessiveMutex<T> {
        RecessiveMutex {
            wants_access: AtomicBool::new(false),
            mutex: Mutex::new(t),
        }
    }

    pub fn lock_dominant(&self) -> LockResult<MutexGuard<'_, T>> {
        self.wants_access.store(true, atomic::Ordering::SeqCst);

        let guard = self.mutex.lock();

        self.wants_access.store(false, atomic::Ordering::SeqCst);

        guard
    }

    pub fn lock_recessive(&self) -> LockResult<MutexGuard<'_, T>> {
        //Yield if we should
        //If we don't do this then the dominant thread will have a hard time competing for the mutex
        while self.wants_access.load(atomic::Ordering::SeqCst) {
            std::thread::yield_now();
        }

        self.mutex.lock()
    }
}

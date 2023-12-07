use std::{
    sync::{
        atomic::{self, AtomicBool},
        Arc, LockResult, MutexGuard,
    },
    thread::JoinHandle,
};

use crate::util::RecessiveMutex;

use super::{KeyCracker, KeyCrackerSampleProvider, KeyCrackerSettings};

pub(super) struct KeyCrackerThread<'a> {
    thread: Option<JoinHandle<()>>,
    should_exit: Arc<AtomicBool>,
    state: Arc<RecessiveMutex<KeyCracker<'a>>>,
}

impl<'d> KeyCrackerThread<'d> {
    fn cracker_thread_func(should_exit: &AtomicBool, state: &RecessiveMutex<KeyCracker<'d>>) {
        while !should_exit.load(atomic::Ordering::SeqCst) {
            //Lock the cracker state
            let Ok(mut state) = state.lock_recessive() else {
                //The main thread crashed while holding the lock - exit as well
                return;
            };

            //Do one unit of work
            if state.is_running() {
                state.do_work();
            } else {
                //Indicate we're exiting cleanly
                should_exit.store(true, atomic::Ordering::SeqCst);
                return;
            }
        }
    }

    pub fn launch(
        settings: KeyCrackerSettings,
        sample_provider: &'d mut KeyCrackerSampleProvider,
    ) -> KeyCrackerThread<'d> {
        //Create the thread state
        let should_exit = Arc::new(AtomicBool::new(false));
        let state = Arc::new(RecessiveMutex::new(KeyCracker::new(
            settings,
            sample_provider,
        )));

        //Launch the key cracker thread
        let thread = {
            //We know the thread is joined in the drop method, so the thread
            //will drop the Arc before 'a goes out of scope (since
            //CrackerThread can not live longer than 'a)
            let should_exit = should_exit.clone();
            let state = unsafe {
                std::mem::transmute::<_, Arc<RecessiveMutex<KeyCracker<'static>>>>(state.clone())
            };

            std::thread::spawn(move || {
                let state =
                    unsafe { std::mem::transmute::<_, Arc<RecessiveMutex<KeyCracker<'d>>>>(state) };

                Self::cracker_thread_func(&should_exit, &state);
            })
        };

        KeyCrackerThread {
            thread: Some(thread),
            should_exit,
            state,
        }
    }

    pub fn did_crash(&self) -> bool {
        !self.should_exit.load(atomic::Ordering::SeqCst)
            && match self.thread.as_ref() {
                Some(thread) => thread.is_finished(),
                None => true,
            }
    }

    pub fn lock_state(&self) -> LockResult<MutexGuard<'_, KeyCracker<'d>>> {
        self.state.lock_dominant()
    }
}

impl Drop for KeyCrackerThread<'_> {
    fn drop(&mut self) {
        //Stop the key cracker thread
        self.should_exit.store(true, atomic::Ordering::SeqCst);

        //Join on the crack thread, and propagate panics
        if let Some(handle) = self.thread.take() {
            if let Err(err) = handle.join() {
                std::panic::resume_unwind(err);
            }
        }
    }
}

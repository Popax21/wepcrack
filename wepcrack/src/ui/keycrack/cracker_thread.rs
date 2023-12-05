use std::{
    sync::{Arc, LockResult, MutexGuard},
    thread::JoinHandle,
};

use crate::{
    keycracker::{KeyCrackerSettings, KeystreamSampleProvider, WepKeyCracker},
    util::RecessiveMutex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyCrackPhase {
    SampleCollection,
    KeyTesting,
    Done,
}

pub(crate) struct KeyCrackerThreadData<'a> {
    exit: bool,
    phase: KeyCrackPhase,

    pub cracker: WepKeyCracker,
    pub sample_provider: &'a KeystreamSampleProvider,
}

impl KeyCrackerThreadData<'_> {
    pub const fn phase(&self) -> KeyCrackPhase {
        self.phase
    }

    pub fn change_phase(&mut self, phase: KeyCrackPhase) {
        self.phase = phase;
    }
}

pub(crate) struct KeyCrackerThread<'a> {
    data: Arc<RecessiveMutex<KeyCrackerThreadData<'a>>>,
    thread: Option<JoinHandle<()>>,
}

impl<'d> KeyCrackerThread<'d> {
    fn cracker_thread_func(data: &RecessiveMutex<KeyCrackerThreadData<'d>>) {
        loop {
            //Lock the cracker data
            let Ok(mut cracker_data) = data.lock_recessive() else {
                return;
            };

            //Exit if we should
            if cracker_data.exit {
                return;
            }

            //Run per-phase logic
            match cracker_data.phase {
                KeyCrackPhase::SampleCollection => {
                    //Collect a sample and process it
                    let sample = (cracker_data.sample_provider)();
                    cracker_data.cracker.accept_sample(&sample);
                }
                KeyCrackPhase::KeyTesting => {
                    //TODO
                }
                KeyCrackPhase::Done => std::thread::yield_now(),
            }
        }
    }

    pub fn launch(
        cracker_settings: &KeyCrackerSettings,
        sample_provider: &'d KeystreamSampleProvider,
    ) -> KeyCrackerThread<'d> {
        //Initialize the key cracker data
        let data = KeyCrackerThreadData::<'d> {
            exit: false,
            phase: KeyCrackPhase::SampleCollection,

            cracker: WepKeyCracker::new(cracker_settings),
            sample_provider,
        };
        let data = Arc::new(RecessiveMutex::new(data));

        //Launch the key cracker thread
        let thread = {
            let data = unsafe {
                //We know the thread is joined in the drop method, so the thread
                //will drop the Arc before 'a goes out of scope (since
                //CrackerThread can not live longer than 'a)
                std::mem::transmute::<_, Arc<RecessiveMutex<KeyCrackerThreadData<'static>>>>(
                    data.clone(),
                )
            };

            std::thread::spawn(move || {
                let data = unsafe {
                    std::mem::transmute::<_, Arc<RecessiveMutex<KeyCrackerThreadData<'d>>>>(data)
                };
                Self::cracker_thread_func(&data);
            })
        };

        KeyCrackerThread {
            data,
            thread: Some(thread),
        }
    }

    pub fn did_crash(&self) -> bool {
        match self.thread.as_ref() {
            Some(thread) => thread.is_finished(),
            None => true,
        }
    }

    pub fn lock_data(&self) -> LockResult<MutexGuard<'_, KeyCrackerThreadData<'d>>> {
        self.data.lock_dominant()
    }
}

impl Drop for KeyCrackerThread<'_> {
    fn drop(&mut self) {
        //Stop the key cracker thread
        if let Ok(mut data) = self.data.lock_dominant() {
            data.exit = true;
        }

        //Join on the crack thread, and propagate panics
        if let Some(handle) = self.thread.take() {
            if let Err(err) = handle.join() {
                std::panic::resume_unwind(err);
            }
        }
    }
}

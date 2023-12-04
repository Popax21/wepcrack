use std::{
    sync::{Arc, LockResult, MutexGuard},
    thread::JoinHandle,
};

use crate::{
    keycracker::{KeyCrackerSettings, KeystreamSampleProvider, WepKeyCracker},
    util::RecessiveMutex,
};

pub(crate) struct KeyCrackerThreadData<'a> {
    exit: bool,
    pub cracker: WepKeyCracker,
    pub sample_provider: &'a KeystreamSampleProvider,
}

pub(crate) struct KeyCrackerThread<'a> {
    data: Arc<RecessiveMutex<KeyCrackerThreadData<'a>>>,
    thread: Option<JoinHandle<()>>,
}

impl<'d> KeyCrackerThread<'d> {
    pub fn launch<'a>(
        cracker_settings: &KeyCrackerSettings,
        sample_provider: &'a KeystreamSampleProvider,
    ) -> KeyCrackerThread<'a> {
        //Initialize the key cracker data
        let data = KeyCrackerThreadData {
            exit: false,
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

            std::thread::spawn(move || loop {
                //Lock the cracker data
                let mut cracker_data = data.lock_recessive().unwrap();

                //Exit if we should
                if cracker_data.exit {
                    return;
                }

                //Collect a sample and process it
                let sample = (cracker_data.sample_provider)();
                cracker_data.cracker.accept_sample(&sample);
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

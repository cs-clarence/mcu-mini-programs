use std::{
    fmt::Debug,
    fs::{self, File},
    mem,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::util::{
    result::Error,
    sync::{arc_sync_mutex, ArcSyncMutex},
};

use crate::util::sync::IntoSendSync;

pub trait Storage<T: Default> {
    type Error: Debug;

    fn save(&self, item: &T) -> Result<(), Self::Error>;
    fn load(&self) -> Result<Option<T>, Self::Error>;
}

type Subscriber<C> = Box<dyn FnMut(&C, &C) + Send + 'static>;

pub struct PersistentStateManager<C: Default, S: Storage<C>> {
    state: C,
    storage: S,
    subscribers: Vec<Subscriber<C>>,
}

pub type SendSyncPersistentStateManager<C, S> = ArcSyncMutex<PersistentStateManager<C, S>>;

impl<C: Default, S: Storage<C>> IntoSendSync for PersistentStateManager<C, S>
where
    SendSyncPersistentStateManager<C, S>: Send + Sync,
{
    type SendSync = SendSyncPersistentStateManager<C, S>;

    fn into_send_sync(self) -> Self::SendSync {
        arc_sync_mutex(self)
    }
}

impl<C: Default, S: Storage<C>> PersistentStateManager<C, S> {
    pub fn update_state(&mut self, f: impl FnOnce(&C) -> C) -> Result<(), S::Error> {
        let new_state = f(self.state());
        let old_state = mem::replace(self.state_mut(), new_state);
        self.write_state()?;

        self.notify_subs(&old_state);

        Ok(())
    }

    pub fn set_state(&mut self, new_state: C) -> Result<(), S::Error> {
        let old_state = mem::replace(self.state_mut(), new_state);
        self.write_state()?;

        self.notify_subs(&old_state);

        Ok(())
    }

    fn notify_subs(&mut self, old_state: &C) {
        for subscriber in self.subscribers.iter_mut() {
            subscriber(old_state, &self.state);
        }
    }

    pub fn state(&self) -> &C {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut C {
        &mut self.state
    }

    fn write_state(&mut self) -> Result<(), S::Error> {
        self.storage.save(&self.state)?;

        Ok(())
    }

    pub fn new(config: C, storage: S) -> Self {
        Self {
            state: config,
            storage,
            subscribers: Vec::new(),
        }
    }

    /// Subscribe to state changes, the callback will be called with the old and new state as arguments
    pub fn subscribe<F: FnMut(&C, &C) + Send + 'static>(&mut self, f: F) {
        self.subscribers.push(Box::new(f));
    }

    pub fn new_loaded_or_default(storage: S) -> Result<Self, S::Error> {
        let value = storage.load()?.unwrap_or_else(|| {
            let def = C::default();
            let res = storage.save(&def);

            if let Err(e) = res {
                tracing::error!("Failed to save default state: {:?}", e);
            }

            def
        });

        Ok(Self::new(value, storage))
    }
}

pub struct BinaryFileStorage<State: Serialize + for<'a> Deserialize<'a>> {
    path: PathBuf,
    _phantom: std::marker::PhantomData<State>,
}

impl<Config: Serialize + for<'a> Deserialize<'a>> BinaryFileStorage<Config> {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Config: Serialize + for<'a> Deserialize<'a> + Default> Storage<Config>
    for BinaryFileStorage<Config>
{
    type Error = Error;

    fn save(&self, item: &Config) -> Result<(), Self::Error> {
        let f = File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.path)?;

        ciborium::into_writer(item, f)?;

        Ok(())
    }

    fn load(&self) -> Result<Option<Config>, Self::Error> {
        if self.path.exists() {
            let value = fs::read(&self.path)?;

            let config = ciborium::from_reader(&value[..]);
            if let Ok(conf) = config {
                return Ok(Some(conf));
            }
        }

        Ok(None)
    }
}

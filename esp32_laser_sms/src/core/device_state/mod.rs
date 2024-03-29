use std::borrow::BorrowMut;

use serde::{Deserialize, Serialize};

use crate::util::{
    result::{error, Result},
    sync::{arc_sync_mutex, ArcSyncMutex},
};

use super::persistent_state;
use crate::util::sync::IntoSendSync;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Default)]
pub enum Mode {
    #[default]
    Pair,
    Connected,
}

#[inline(always)]
fn default_name() -> String {
    "Feeder Device".to_string()
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DeviceState {
    #[serde(default = "default_name")]
    pub name: String,
    pub mode: Mode,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            name: default_name(),
            mode: Mode::Pair,
        }
    }
}

impl DeviceState {
    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
}

pub type DeviceStateManager<S = DeviceStateStorage> =
    persistent_state::PersistentStateManager<DeviceState, S>;

pub type SendSyncDeviceStateManager<S = DeviceStateStorage> =
    persistent_state::SendSyncPersistentStateManager<DeviceState, S>;

pub type DeviceStateStorage = persistent_state::BinaryFileStorage<DeviceState>;

pub type SendSyncDeviceStateService<S = DeviceStateStorage, M = DeviceStateManager<S>> =
    ArcSyncMutex<DeviceStateService<S, M>>;

pub struct DeviceStateService<
    S: persistent_state::Storage<DeviceState>,
    M: BorrowMut<DeviceStateManager<S>>,
> {
    state_manager: M,
    _state: std::marker::PhantomData<S>,
}

impl<S: persistent_state::Storage<DeviceState>, M: BorrowMut<DeviceStateManager<S>>>
    DeviceStateService<S, M>
{
    pub fn new(state_manager: M) -> Result<Self> {
        Ok(Self {
            state_manager,
            _state: std::marker::PhantomData,
        })
    }

    pub fn set_mode(&mut self, mode: Mode) -> Result<()> {
        self.state_manager
            .borrow_mut()
            .update_state(|state| {
                let mut state = state.clone();
                state.set_mode(mode);

                state
            })
            .map_err(|e| error!("{:?}", e))?;

        Ok(())
    }

    /// Subscribe to state changes, the callback will be called with the old and new state as arguments
    pub fn subscribe(
        &mut self,
        callback: impl FnMut(&DeviceState, &DeviceState) + Send + Sync + 'static,
    ) {
        self.state_manager.borrow_mut().subscribe(callback);
    }

    pub fn set_name(&mut self, name: String) -> Result<()> {
        self.state_manager
            .borrow_mut()
            .update_state(|state| {
                let mut state = state.clone();
                state.name = name;
                state
            })
            .map_err(|e| error!("{:?}", e))?;

        Ok(())
    }

    pub fn mode(&mut self) -> Mode {
        self.state_manager.borrow_mut().state().mode()
    }

    pub fn name(&mut self) -> &str {
        &self.state_manager.borrow_mut().state().name
    }
}

impl<S, M> IntoSendSync for DeviceStateService<S, M>
where
    S: persistent_state::Storage<DeviceState>,
    M: BorrowMut<DeviceStateManager<S>>,
    SendSyncDeviceStateService<S, M>: Send + Sync,
{
    type SendSync = SendSyncDeviceStateService<S, M>;

    fn into_send_sync(self) -> Self::SendSync {
        arc_sync_mutex(self)
    }
}

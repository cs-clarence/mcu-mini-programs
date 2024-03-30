use std::borrow::BorrowMut;

use serde::{Deserialize, Serialize};
use time::{macros::time, Time};

use crate::util::{
    result::{self, error, Result},
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Eq)]
pub struct DeviceState {
    pub sms_send_phone_number: Option<String>,
    pub sms_send_throttle: u64,
    pub sms_send_twilio_phone_number: Option<String>,
    pub sms_send_twilio_account_sid: Option<String>,
    pub sms_send_twilio_auth_token: Option<String>,
    pub activation_time_start: Time,
    pub activation_time_end: Option<Time>,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            sms_send_phone_number: None,
            sms_send_throttle: 60_000, // 1 sms per minute
            sms_send_twilio_phone_number: None,
            sms_send_twilio_account_sid: None,
            sms_send_twilio_auth_token: None,
            activation_time_start: time!(20:00:00),
            activation_time_end: Some(time!(00:00:00)),
        }
    }
}

impl DeviceState {
    pub fn sms_send_phone_number(&self) -> Option<&str> {
        self.sms_send_phone_number.as_deref()
    }

    pub fn sms_send_thottle(&self) -> u64 {
        self.sms_send_throttle
    }

    pub fn sms_send_twilio_phone_number(&self) -> Option<&str> {
        self.sms_send_twilio_phone_number.as_deref()
    }

    pub fn sms_send_twilio_account_sid(&self) -> Option<&str> {
        self.sms_send_twilio_account_sid.as_deref()
    }

    pub fn sms_send_twilio_auth_token(&self) -> Option<&str> {
        self.sms_send_twilio_auth_token.as_deref()
    }

    pub fn activation_time_start(&self) -> &Time {
        &self.activation_time_start
    }

    pub fn activation_time_end(&self) -> Option<&Time> {
        self.activation_time_end.as_ref()
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

    /// Subscribe to state changes, the callback will be called with the old and new state as arguments
    pub fn subscribe(
        &mut self,
        callback: impl FnMut(&DeviceState, &DeviceState) + Send + Sync + 'static,
    ) {
        self.state_manager.borrow_mut().subscribe(callback);
    }

    fn update_state(&mut self, f: impl FnOnce(&DeviceState) -> DeviceState) -> Result<()> {
        self.state_manager
            .borrow_mut()
            .update_state(f)
            .map_err(|e| error!("Error: {:?}", e))
    }

    pub fn sms_send_phone_number(&self) -> Option<&str> {
        self.state_manager.borrow().state().sms_send_phone_number()
    }

    pub fn set_sms_send_phone_number(&mut self, phone_number: &str) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.sms_send_phone_number = Some(phone_number.to_string());

            c
        })
    }

    pub fn sms_send_throttle(&self) -> u64 {
        self.state_manager.borrow().state().sms_send_thottle()
    }

    pub fn set_sms_send_throttle(&mut self, throttle: u64) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.sms_send_throttle = throttle;
            c
        })
    }

    pub fn sms_send_twilio_phone_number(&self) -> Option<&str> {
        self.state_manager
            .borrow()
            .state()
            .sms_send_twilio_phone_number()
    }

    pub fn set_sms_send_twilio_phone_number(&mut self, phone_number: &str) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.sms_send_twilio_phone_number = Some(phone_number.to_string());
            c
        })
    }

    pub fn sms_send_twilio_account_sid(&self) -> Option<&str> {
        self.state_manager
            .borrow()
            .state()
            .sms_send_twilio_account_sid()
    }

    pub fn set_sms_send_twilio_account_sid(&mut self, account_sid: &str) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.sms_send_twilio_account_sid = Some(account_sid.to_string());
            c
        })
    }

    pub fn sms_send_twilio_auth_token(&self) -> Option<&str> {
        self.state_manager
            .borrow()
            .state()
            .sms_send_twilio_auth_token()
    }

    pub fn set_sms_send_twilio_auth_token(&mut self, auth_token: &str) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.sms_send_twilio_auth_token = Some(auth_token.to_string());
            c
        })
    }

    pub fn set_sms(
        &mut self,
        phone_number: Option<&str>,
        throttle: u64,
        twilio_phone_number: Option<&str>,
        account_sid: Option<&str>,
        auth_token: Option<&str>,
    ) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.sms_send_phone_number = phone_number.map(|s| s.to_string());
            c.sms_send_throttle = throttle;
            c.sms_send_twilio_phone_number = twilio_phone_number.map(|s| s.to_string());
            c.sms_send_twilio_account_sid = account_sid.map(|s| s.to_string());
            c.sms_send_twilio_auth_token = auth_token.map(|s| s.to_string());
            c
        })
    }

    pub fn activation_time_start(&self) -> &Time {
        self.state_manager.borrow().state().activation_time_start()
    }

    pub fn set_activation_time_start(&mut self, time_start: Time) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.activation_time_start = time_start;
            c
        })
    }

    pub fn activation_time_end(&self) -> Option<&Time> {
        self.state_manager.borrow().state().activation_time_end()
    }

    pub fn set_activation_time_end(&mut self, time_end: Time) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.activation_time_end = Some(time_end);
            c
        })
    }

    pub fn set_activation(&mut self, time_start: &Time, time_end: &Time) -> result::Result<()> {
        self.update_state(|state| {
            let mut c = state.clone();
            c.activation_time_start = *time_start;
            c.activation_time_end = Some(*time_end);
            c
        })
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

use std::collections::HashSet;

use crate::core::{
  device_state::SendSyncDeviceStateService,
  dispenser::SendSyncDispenserService,
  level::SendSyncLevelService,
  meal_plans::MealPlan,
  persistent_state::{BinaryFileStorage, SendSyncPersistentStateManager},
  wifi::SendSyncWifi,
};

#[derive(Clone)]
pub struct ServerState {
  pub wifi: SendSyncWifi,
  pub device_state: SendSyncDeviceStateService,
  pub dispenser: SendSyncDispenserService<'static>,
  pub meal_plans: SendSyncPersistentStateManager<
    HashSet<MealPlan>,
    BinaryFileStorage<HashSet<MealPlan>>,
  >,
  pub level: SendSyncLevelService<'static>,
}

impl ServerState {
  pub fn new(
    wifi: SendSyncWifi,
    device_state: SendSyncDeviceStateService,
    dispenser: SendSyncDispenserService<'static>,
    meal_plans: SendSyncPersistentStateManager<
      HashSet<MealPlan>,
      BinaryFileStorage<HashSet<MealPlan>>,
    >,
    level: SendSyncLevelService<'static>,
  ) -> Self {
    Self {
      wifi,
      device_state,
      dispenser,
      meal_plans,
      level,
    }
  }
}

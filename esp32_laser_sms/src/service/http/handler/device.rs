use axum::{
  debug_handler,
  routing::{get, patch, post},
  Extension, Json, Router,
};
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  core::device::Device,
  service::{
    http::common::response::{data, DataReponse, ErrorResponse},
    state::ServerState,
  },
};

#[derive(Serialize)]
struct DeviceInfo {
  id: Uuid,
  name: String,
}

#[derive(Serialize)]
struct GetDeviceInfoData {
  device_info: DeviceInfo,
}

#[debug_handler]
async fn get_info(
  Extension(state): Extension<ServerState>,
) -> Result<DataReponse<GetDeviceInfoData>, ErrorResponse> {
  let mut state_svc = state.device_state.lock();

  Ok(data(GetDeviceInfoData {
    device_info: DeviceInfo {
      id: state_svc.id(),
      name: state_svc.name().to_string(),
    },
  }))
}

#[debug_handler]
async fn reset() {
  defer! {
    _ = Device::reset();
  };
}

#[debug_handler]
async fn restart() {
  defer! {
    Device::restart();
  }
}

#[derive(Deserialize)]
struct SetNameData {
  name: String,
}

#[debug_handler]
async fn set_name(
  Extension(state): Extension<ServerState>,
  Json(data): Json<SetNameData>,
) -> Result<(), ErrorResponse> {
  state.device_state.lock().set_name(data.name)?;

  Ok(())
}

pub fn build_router() -> Router {
  Router::new()
    .route("/info", get(get_info))
    .route("/reset", post(reset))
    .route("/restart", post(restart))
    .route("/name", patch(set_name))
}

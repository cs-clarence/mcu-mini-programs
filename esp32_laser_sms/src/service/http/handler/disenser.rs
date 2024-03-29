use axum::{
  routing::{get, post},
  Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::service::{
  http::common::response::{data, DataReponse, ErrorResponse},
  state::ServerState,
};

#[derive(Deserialize)]
struct DispenseRequest {
  servings: u64,
}

#[axum::debug_handler]
#[instrument(skip_all, err)]
async fn dispense(
  Extension(state): Extension<ServerState>,
  Json(req): Json<DispenseRequest>,
) -> Result<(), ErrorResponse> {
  state
    .dispenser
    .try_lock()
    .map_err(|_| ErrorResponse::new("Device is busy"))?
    .dispense(req.servings)
    .await?;

  Ok(())
}

#[derive(Serialize)]
struct DispenserLevelData {
  level: u8,
}

#[axum::debug_handler]
#[instrument(skip_all, err)]
async fn level(
  Extension(state): Extension<ServerState>,
) -> Result<DataReponse<DispenserLevelData>, ErrorResponse> {
  let level = state
    .level
    .try_lock()
    .map_err(|_| ErrorResponse::new("Device is busy"))?
    .read()?;

  Ok(data(DispenserLevelData { level }))
}

pub fn build_router() -> Router {
  Router::new()
    .route("/dispense", post(dispense))
    .route("/level", get(level))
}

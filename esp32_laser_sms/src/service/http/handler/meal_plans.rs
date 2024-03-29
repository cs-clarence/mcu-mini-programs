use axum::{
  debug_handler,
  extract::Path,
  routing::{patch, post},
  Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  core::meal_plans::{MealPlan, MealPlans},
  service::{
    http::common::response::{
      data, success, DataReponse, ErrorResponse, SuccessResponse,
    },
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
async fn add_meal_plan(
  Extension(state): Extension<ServerState>,
  Json(meal_plan): Json<MealPlan>,
) -> Result<SuccessResponse, ErrorResponse> {
  let mut meal_plans = state.meal_plans.lock();
  let current = meal_plans.state();

  let existing = current.iter().find(|i| meal_plan.id == i.id);

  if existing.is_some() {
    return Err(ErrorResponse::new("Meal plan with the id already exists"));
  }

  meal_plans.update_state(|conf| {
    let mut conf = conf.clone();
    conf.insert(meal_plan);
    conf
  })?;

  Ok(success("Meal plan added successfully"))
}

#[derive(Serialize, Deserialize)]
struct MealPlanIdParams {
  #[serde(rename = "meal-plan-id")]
  meal_plan_id: String,
}

#[debug_handler]
async fn update_meal_plan(
  Extension(state): Extension<ServerState>,
  Path(MealPlanIdParams { meal_plan_id }): Path<MealPlanIdParams>,
  Json(meal_plan): Json<MealPlan>,
) -> Result<SuccessResponse, ErrorResponse> {
  let mut meal_plans = state.meal_plans.lock();
  let current = meal_plans.state();

  let existing = current.iter().find(|i| meal_plan_id == i.id).cloned();

  if let Some(existing) = existing {
    meal_plans.update_state(|conf| {
      let mut conf = conf.clone();
      conf.remove(&existing);
      conf.insert(meal_plan);

      conf
    })?;
  } else {
    return Err(ErrorResponse::new("Meal plan with the id does not exist"));
  }

  Ok(success("Meal plan updated successfully"))
}

#[debug_handler]
async fn get_meal_plans(
  Extension(state): Extension<ServerState>,
) -> Result<DataReponse<MealPlans>, ErrorResponse> {
  let meal_plans = state.meal_plans.lock();
  let current = meal_plans.state();

  Ok(data(current.clone()))
}

#[debug_handler]
async fn get_meal_plan(
  Extension(state): Extension<ServerState>,
  Path(MealPlanIdParams { meal_plan_id }): Path<MealPlanIdParams>,
) -> Result<DataReponse<MealPlan>, ErrorResponse> {
  let meal_plans = state.meal_plans.lock();
  let current = meal_plans.state();

  let existing = current.iter().find(|i| meal_plan_id == i.id).cloned();

  if let Some(existing) = existing {
    Ok(data(existing))
  } else {
    Err(ErrorResponse::new("Meal plan with the id does not exist"))
  }
}

#[debug_handler]
async fn delete_meal_plan(
  Extension(state): Extension<ServerState>,
  Path(MealPlanIdParams { meal_plan_id }): Path<MealPlanIdParams>,
) -> Result<SuccessResponse, ErrorResponse> {
  let mut meal_plans = state.meal_plans.lock();
  let current = meal_plans.state();

  let existing = current.iter().find(|i| meal_plan_id == i.id).cloned();

  if let Some(existing) = existing {
    meal_plans.update_state(|conf| {
      let mut conf = conf.clone();
      conf.remove(&existing);

      conf
    })?;
  } else {
    return Err(ErrorResponse::new("Meal plan with the id does not exist"));
  }

  Ok(success("Meal plan deleted successfully"))
}

pub fn build_router() -> Router {
  Router::new()
    .route("/", post(add_meal_plan).get(get_meal_plans))
    .route(
      "/:meal-plan-id",
      patch(update_meal_plan)
        .delete(delete_meal_plan)
        .get(get_meal_plan),
    )
}

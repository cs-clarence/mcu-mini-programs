use axum::Router;
use tower::ServiceBuilder;
use tower_http::{
  add_extension::AddExtensionLayer,
  catch_panic::CatchPanicLayer,
  cors::{
    AllowCredentials, AllowHeaders, AllowMethods, AllowOrigin, CorsLayer,
  },
  trace::TraceLayer,
};

use crate::service::state::ServerState;

use super::handler::*;

pub fn build(state: ServerState) -> Router {
  let layers = ServiceBuilder::new()
    .layer(CatchPanicLayer::new())
    .layer(TraceLayer::new_for_http())
    .layer(AddExtensionLayer::new(state))
    .layer(
      CorsLayer::new()
        .allow_credentials(AllowCredentials::yes())
        .allow_methods(AllowMethods::mirror_request())
        .allow_origin(AllowOrigin::mirror_request())
        .allow_headers(AllowHeaders::mirror_request()),
    );

  Router::new()
    .nest("/ping", ping::build_router())
    .nest("/wifi", wifi::build_router())
    .nest("/ble", ble::build_router())
    .nest("/device", device::build_router())
    .nest("/dispenser", disenser::build_router())
    .nest("/meal-plans", meal_plans::build_router())
    .layer(layers)
}

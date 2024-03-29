use axum::{
  debug_handler,
  extract::Query,
  routing::{get, post},
  Extension, Json, Router,
};
use embedded_svc::wifi::AccessPointInfo;
use scopeguard::guard;
use serde::{Deserialize, Serialize};

use crate::{
  core::{device_state::Mode, wifi::Credential},
  service::{
    http::common::response::{
      data, success, DataReponse, ErrorResponse, SuccessResponse,
    },
    state::ServerState,
  },
  util::{
    delay::non_blocking::{self, delay_ms},
    result,
  },
};

#[derive(Serialize)]
struct GetAccessPointsData {
  access_points: Vec<AccessPointInfo>,
}

#[debug_handler]
async fn scan_access_points(
  Extension(state): Extension<ServerState>,
) -> Result<DataReponse<GetAccessPointsData>, ErrorResponse> {
  let access_points = state.wifi.lock().await.scan_aps().await?;

  Ok(data(GetAccessPointsData { access_points }))
}

#[derive(Deserialize, Clone)]
struct ForgetWifiCredentialsData {
  ssid: String,
  bssid: Option<[u8; 6]>,
}

#[debug_handler]
async fn forget_wifi_credential(
  Query(data): Query<ForgetWifiCredentialsData>,
  Extension(state): Extension<ServerState>,
) -> Result<SuccessResponse, ErrorResponse> {
  let wifi = state.wifi.clone();
  let device_state = state.device_state.clone();

  scopeguard::defer! {
    let data = data.clone();
    tokio::spawn(async move {
      let mut wifi = wifi.lock_owned().await;
      non_blocking::delay_ms(1000).await;
      if wifi.is_connected()? {
        let ap = wifi.sta_ap_info()?;
        let should_reconnect = if let Some(bssid) = data.bssid {
          bssid == ap.bssid && data.ssid == ap.ssid.as_str()
        } else {
          data.ssid == ap.ssid.as_str()
        };

        if should_reconnect {
          let result = wifi.reconnect(5).await;

          let should_pair =
            result.map(|did_connect| !did_connect).ok().unwrap_or(true);
          if should_pair {
            device_state.lock().set_mode(Mode::Pair)?;
          }
        }
      }

      result::Ok(())
    });
  };

  let mut wifi = state.wifi.lock().await;

  wifi.forget_ap(&data.ssid, data.bssid)?;

  Ok(success("Successfully deleted access point credential"))
}

#[debug_handler]
async fn get_saved_credentials(
  Extension(state): Extension<ServerState>,
) -> Result<DataReponse<Vec<Credential>>, ErrorResponse> {
  let credentials = state.wifi.lock().await.saved_credentials()?;
  Ok(data(credentials))
}

#[derive(Deserialize)]
struct ConnectRequest {
  ssid: String,
  bssid: Option<[u8; 6]>,
  psk: String,
  retries: Option<u8>,
}

#[debug_handler]
async fn connect(
  Extension(state): Extension<ServerState>,
  Json(request): Json<ConnectRequest>,
) -> Result<SuccessResponse, ErrorResponse> {
  let ConnectRequest {
    ssid,
    psk,
    retries,
    bssid,
  } = request;
  let retries = retries.unwrap_or(0);

  let mut wifi = guard(state.wifi.lock_owned().await, |mut wifi| {
    if wifi.is_ap_enabled().expect("Failed to check AP mode") {
      tokio::spawn(async move {
        // This should be enough time for the device to connect to the AP
        delay_ms(5000).await;
        if let Err(err) = wifi.switch_to_sta_only().await {
          tracing::error!("Failed to switch to STA only: {}", err);
        } else {
          tracing::info!("Successfully switched to STA only");
        }
      });
    }
  });

  wifi.connect(&ssid, &psk, None, None, retries).await?;

  let bssid = if let Some(bssid) = bssid {
    bssid
  } else {
    wifi.sta_ap_info()?.bssid
  };
  state.device_state.lock().set_mode(Mode::Connected)?;

  wifi.save_ap_credential(&ssid, &psk, bssid)?;

  Ok(success("Successfully connected to access point"))
}

pub fn build_router() -> Router {
  Router::new()
    .route("/access-points/scan", get(scan_access_points))
    .route("/access-points/credentials", get(get_saved_credentials))
    .route(
      "/access-points/credentials/forget",
      post(forget_wifi_credential),
    )
    .route("/connect", post(connect))
}

#![allow(dead_code)]
#![feature(decl_macro)]
#![feature(lazy_cell)]
#![feature(try_blocks)]

use core::{
    device::Device,
    wifi::{self, Wifi},
};
use std::rc::Rc;

pub mod core;
pub mod service;
pub mod util;

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{gpio::PinDriver, peripherals::Peripherals, task::block_on},
    http::{
        server::{Configuration, EspHttpServer},
        Method,
    },
    io::Write,
    nvs::EspDefaultNvsPartition,
    sntp::{EspSntp, OperatingMode, SntpConf, SyncMode},
    sys,
    timer::EspTaskTimerService,
};
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use time::{
    ext::NumericalStdDuration,
    format_description::FormatItem,
    macros::{format_description, offset, time},
    OffsetDateTime, Time, UtcOffset,
};
use util::{
    result::{Ok, Result},
    tracing,
};

use crate::{
    core::device_state,
    util::{delay::blocking::delay_ms, result, sync::IntoSendSync as _},
};

fn get_free_heap_size() -> u32 {
    unsafe { sys::esp_get_free_heap_size() }
}

const OFFSET: UtcOffset = offset!(+8);

const DATE_TIME_FMT: &[FormatItem<'static>] =
    format_description!("[hour repr:12]:[minute] [period], [year]-[month]-[day]");

const TIME_FMT: &[FormatItem<'static>] = format_description!("[hour repr:12]:[minute] [period]");

const HTTP_SERVER_STACK_SIZE: usize = 32 * 1024;

time::serde::format_description!(time_de, Time, FMT);
const FMT: &[FormatItem<'_>] = format_description!("[hour repr:24]:[minute][optional [:[second]]]");

static mut TIME_SYNCED: bool = false;

async fn async_main<'a>() -> Result<()> {
    tracing::info!("START: Memory [heap: {} free bytes]", get_free_heap_size(),);

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;
    let ldr_photoresistor_pin = pins.gpio32;
    // let piezo_buzzer_pin = pins.gpio33;

    let ldr_photoresistor = Rc::new(PinDriver::input(ldr_photoresistor_pin)?);

    let storage = device_state::DeviceStateStorage::new("/spiflash/conf/device.bin");
    let cfg = device_state::DeviceStateManager::new_loaded_or_default(storage)?;
    let dev_svc = device_state::DeviceStateService::new(cfg)?.into_send_sync();

    let modem = peripherals.modem;
    let sys_loop = EspSystemEventLoop::take()?;

    let timer_service = EspTaskTimerService::new()?;

    let storage = wifi::WifiStateStorage::new("/spiflash/conf/wifi.bin");
    let manager = wifi::WifiStateManager::new_loaded_or_default(storage)?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = Wifi::new(modem, sys_loop, timer_service, Some(nvs), manager)?;
    if !wifi.reconnect(5).await? {
        wifi.start_ap_default().await?;
    }

    let wifi = wifi.into_send_sync();

    let _sntp = EspSntp::new_with_callback(
        &SntpConf {
            sync_mode: SyncMode::Immediate,
            operating_mode: OperatingMode::Poll,
            servers: ["121.58.193.100"], // ntp.pagasa.dost.gov.ph
        },
        |_| unsafe {
            TIME_SYNCED = true;
        },
    )?;

    tracing::info!(
        "BEFORE JOIN: Memory [heap: {} free bytes]",
        get_free_heap_size(),
    );

    let mut server = EspHttpServer::new(&Configuration {
        stack_size: HTTP_SERVER_STACK_SIZE,
        ..Default::default()
    })?;

    server.fn_handler::<result::Error, _>("/", Method::Get, |req| {
        let mut res = req.into_ok_response()?;
        res.write_all(include_bytes!("./html/index.html"))?;

        Ok(())
    })?;

    {
        #[derive(Deserialize)]
        struct ConnectRequest {
            ssid: String,
            bssid: Option<[u8; 6]>,
            psk: String,
            retries: Option<u8>,
        }

        #[derive(Serialize)]
        struct ConnectResponse {
            new_ip: String,
        }

        let wifi = wifi.clone();
        server.fn_handler::<result::Error, _>(
            "/wifi-credentials",
            Method::Post,
            move |mut req| {
                let mut buff = [0u8; 2 * 1024];

                let end = req.read(&mut buff)?;

                let connect_req: ConnectRequest = serde_json::from_slice(&buff[..end])?;

                let mut wifi = wifi.lock();
                block_on(async {
                    wifi.connect(&connect_req.ssid, &connect_req.psk, None, None, 3)
                        .await
                })?;

                let new_ip = wifi.sta_ip_info()?.ip.to_string();
                let bssid = wifi.sta_ap_info()?.bssid;

                wifi.save_ap_credential(&connect_req.ssid, &connect_req.psk, bssid)?;

                let resp = ConnectResponse { new_ip };
                let resp = serde_json::to_string(&resp)?;
                let mut res =
                    req.into_response(200, None, &[("Content-Type", "application/json")])?;

                res.write_all(resp.as_bytes())?;

                defer! {
                    delay_ms(2000);
                    block_on(async {
                        _  = wifi.switch_to_sta_only().await;
                    });
                };

                Ok(())
            },
        )?;
    }

    {
        #[derive(Deserialize)]
        struct SetSmsSendRequest {
            phone_number: Option<String>,
            throttle: u64,
            twilio_phone_number: Option<String>,
            twilio_account_sid: Option<String>,
            twilio_auth_token: Option<String>,
        }

        let dvc = dev_svc.clone();
        server.fn_handler::<result::Error, _>("/sms-send", Method::Post, move |mut req| {
            let mut buff = [0u8; 2 * 1024];

            let end = req.read(&mut buff)?;

            let save_req: SetSmsSendRequest =
                serde_json::from_slice(&buff[..end]).inspect_err(|e| {
                    tracing::error!("Error: {:?}", e);
                })?;

            let mut dvc = dvc.lock();
            dvc.set_sms(
                save_req.phone_number.as_deref(),
                save_req.throttle,
                save_req.twilio_phone_number.as_deref(),
                save_req.twilio_account_sid.as_deref(),
                save_req.twilio_auth_token.as_deref(),
            )
            .inspect_err(|e| {
                tracing::error!("Error: {:?}", e);
            })?;

            req.into_ok_response()?.flush()?;
            Ok(())
        })?;
    }

    {
        #[derive(Deserialize)]
        struct SetActivationRequest {
            #[serde(with = "time_de")]
            time_start: Time,
            #[serde(with = "time_de::option")]
            time_end: Option<Time>,
        }

        let dvc = dev_svc.clone();
        server.fn_handler::<result::Error, _>("/activation", Method::Post, move |mut req| {
            let mut buff = [0u8; 2 * 1024];

            let end = req.read(&mut buff).inspect_err(|e| {
                tracing::error!("Error: {:?}", e);
            })?;

            let set_req: SetActivationRequest =
                serde_json::from_slice(&buff[..end]).inspect_err(|e| {
                    tracing::error!("Error: {:?}", e);
                })?;

            let mut dvc = dvc.lock();
            let time_end = set_req.time_end.unwrap_or(MIDNIGHT);

            dvc.set_activation(&set_req.time_start, &time_end)
                .inspect_err(|e| {
                    tracing::error!("Error: {:?}", e);
                })?;

            req.into_ok_response()?.flush()?;
            Ok(())
        })?;
    }

    {
        #[derive(Serialize)]
        struct GetDeviceInfoResponse {
            sms_send_phone_number: Option<String>,
            sms_send_throttle: u64,
            sms_send_twilio_account_sid: Option<String>,
            sms_send_twilio_auth_token: Option<String>,
            activation_time_start: Time,
            activation_time_end: Option<Time>,
        }

        let dvc = dev_svc.clone();
        server.fn_handler::<result::Error, _>("/device-info", Method::Get, move |req| {
            let dvc = dvc.lock();
            let resp = GetDeviceInfoResponse {
                sms_send_phone_number: dvc.sms_send_phone_number().map(|s| s.to_string()),
                sms_send_throttle: dvc.sms_send_throttle(),
                sms_send_twilio_account_sid: dvc
                    .sms_send_twilio_account_sid()
                    .map(|s| s.to_string()),
                sms_send_twilio_auth_token: dvc.sms_send_twilio_auth_token().map(|s| s.to_string()),
                activation_time_start: *dvc.activation_time_start(),
                activation_time_end: dvc.activation_time_end().copied(),
            };

            let mut res = req.into_response(200, None, &[("Content-Type", "application/json")])?;
            res.write_all(serde_json::to_string(&resp)?.as_bytes())?;

            Ok(())
        })?;
    }

    let mut is_prev_high = false;
    let mut time_sms_last_sent = system_time_now();
    let mut did_trigger_sync_cod = false;
    loop {
        delay_ms(10);
        if unsafe { TIME_SYNCED } && !did_trigger_sync_cod {
            did_trigger_sync_cod = true;
            tracing::info!("Time synced");
        }

        let dvc = dev_svc.lock();
        let is_active = is_active(dvc.activation_time_start(), dvc.activation_time_end());

        if !is_active {
            continue;
        }

        if ldr_photoresistor.is_high() && !is_prev_high {
            tracing::info!("Laser is cut");
            is_prev_high = true;

            let throttle = dvc.sms_send_throttle();
            let now = system_time_now();
            let next_send = time_sms_last_sent + throttle.std_milliseconds();
            let should_send_sms = next_send <= now;
            if should_send_sms {
                time_sms_last_sent = now;
                let twilio = (
                    dvc.sms_send_twilio_phone_number(),
                    dvc.sms_send_twilio_account_sid(),
                    dvc.sms_send_twilio_account_sid(),
                    dvc.sms_send_twilio_auth_token(),
                );

                if let (Some(to), Some(from), Some(sid), Some(auth_token)) = twilio {
                    tracing::info!(
                        "Sending SMS to: {}, from: {}, using {} and {}",
                        to,
                        from,
                        sid,
                        auth_token
                    );
                }
            }
        } else if ldr_photoresistor.is_low() && is_prev_high {
            tracing::info!("Laser is in contact");
            is_prev_high = false;
        }
    }
}

const MIDNIGHT: Time = time!(00:00:00);

fn is_active(time_start: &Time, time_end: Option<&Time>) -> bool {
    let time_now = system_time_now();
    let time_end = time_end.unwrap_or(&MIDNIGHT);

    if time_start < time_end {
        time_now >= *time_start && time_now <= *time_end
    } else {
        time_now >= *time_start || time_now <= *time_end
    }
}

fn system_time_now() -> Time {
    OffsetDateTime::now_utc().to_offset(OFFSET).time()
}

pub fn main() -> eyre::Result<()> {
    esp_idf_svc::sys::link_patches();
    tracing::init()?;
    Device::init()?;
    defer! {
        _ = Device::deinit();
    };

    block_on(async_main())
}

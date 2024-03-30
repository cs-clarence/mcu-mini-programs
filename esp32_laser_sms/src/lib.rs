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

use base64::Engine;
use embedded_svc::http::client::Client as HttpClient;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::PinDriver,
        ledc::{self, LedcDriver, LedcTimerDriver},
        peripherals::Peripherals,
        task::block_on,
        units::FromValueType as _,
    },
    http::{
        client::{self, EspHttpConnection},
        server::{self, EspHttpServer},
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
use urlencoding::encode;
use util::{
    result::{Ok, Result},
    tracing,
};

use crate::{
    core::device_state,
    util::{
        delay::blocking::delay_ms,
        result::{self, bail},
        sync::IntoSendSync as _,
    },
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
    let ldr_photoresistor = Rc::new(PinDriver::input(ldr_photoresistor_pin)?);

    let piezo_buzzer_pin = pins.gpio33;
    let piezo_buzzer_channel = peripherals.ledc.channel0;
    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &ledc::config::TimerConfig {
            frequency: 2200.Hz(),
            speed_mode: ledc::SpeedMode::HighSpeed,
            ..Default::default()
        },
    )?;
    let mut piezo_buzzer = LedcDriver::new(piezo_buzzer_channel, timer_driver, piezo_buzzer_pin)?;

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

    let mut server = EspHttpServer::new(&server::Configuration {
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
                    delay_ms(5000);
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
            message_body: Option<String>,
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
                save_req.message_body.as_deref(),
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
        #[derive(Deserialize)]
        struct SetBuzzerRequest {
            enabled: bool,
        }

        let dvc = dev_svc.clone();
        server.fn_handler::<result::Error, _>("/buzzer", Method::Post, move |mut req| {
            let mut buff = [0u8; 2 * 1024];

            let end = req.read(&mut buff).inspect_err(|e| {
                tracing::error!("Error: {:?}", e);
            })?;

            let set_req: SetBuzzerRequest =
                serde_json::from_slice(&buff[..end]).inspect_err(|e| {
                    tracing::error!("Error: {:?}", e);
                })?;

            let mut dvc = dvc.lock();
            dvc.set_buzzer(set_req.enabled).inspect_err(|e| {
                tracing::error!("Error: {:?}", e);
            })?;

            req.into_ok_response()?.flush()?;
            Ok(())
        })?;
    }

    {
        #[derive(Serialize)]
        struct GetDeviceInfoResponse<'a> {
            sms_send_phone_number: Option<&'a str>,
            sms_send_message_body: Option<&'a str>,
            sms_send_throttle: u64,
            sms_send_twilio_phone_number: Option<&'a str>,
            sms_send_twilio_account_sid: Option<&'a str>,
            sms_send_twilio_auth_token: Option<&'a str>,
            activation_time_start: Time,
            activation_time_end: Option<Time>,
            buzzer_enabled: bool,
        }

        let dvc = dev_svc.clone();
        server.fn_handler::<result::Error, _>("/device-info", Method::Get, move |req| {
            let dvc = dvc.lock();
            let resp = GetDeviceInfoResponse {
                sms_send_phone_number: dvc.sms_send_phone_number(),
                sms_send_message_body: dvc.sms_send_message_body(),
                sms_send_throttle: dvc.sms_send_throttle(),
                sms_send_twilio_phone_number: dvc.sms_send_twilio_phone_number(),
                sms_send_twilio_account_sid: dvc.sms_send_twilio_account_sid(),
                sms_send_twilio_auth_token: dvc.sms_send_twilio_auth_token(),
                activation_time_start: *dvc.activation_time_start(),
                activation_time_end: dvc.activation_time_end().copied(),
                buzzer_enabled: dvc.buzzer_enabled(),
            };

            let mut res = req.into_response(200, None, &[("Content-Type", "application/json")])?;
            res.write_all(serde_json::to_string(&resp)?.as_bytes())?;

            Ok(())
        })?;
    }

    {
        server.fn_handler::<result::Error, _>("/reset-device", Method::Post, move |req| {
            Device::reset()?;

            req.into_ok_response()?.flush()?;
            Ok(())
        })?;

        server.fn_handler::<result::Error, _>("/restart-device", Method::Post, move |req| {
            Device::restart();

            req.into_ok_response()?.flush()?;
            Ok(())
        })?;
    }

    let mut is_prev_high = false;
    let mut time_sms_last_sent =
        system_time_now() - dev_svc.lock().sms_send_throttle().std_milliseconds();
    let mut did_trigger_sync_code = false;
    loop {
        delay_ms(100);
        if unsafe { TIME_SYNCED } && !did_trigger_sync_code {
            tracing::info!("Time synced");
            did_trigger_sync_code = true;
        }
        let photoresistor_is_high = ldr_photoresistor.is_high();

        let dvc = dev_svc.lock();
        let is_active = is_active(dvc.activation_time_start(), dvc.activation_time_end());

        if !is_active {
            piezo_buzzer.set_duty(0)?;
            continue;
        }

        if dvc.buzzer_enabled() && photoresistor_is_high {
            piezo_buzzer.set_duty(100)?;
        } else {
            piezo_buzzer.set_duty(0)?;
        }

        if photoresistor_is_high && !is_prev_high {
            tracing::info!("Laser is cut");
            is_prev_high = true;

            let throttle = dvc.sms_send_throttle();
            let now = system_time_now();
            let next_send = time_sms_last_sent + throttle.std_milliseconds();
            let passed_throttle = next_send <= now;
            let connected_to_wifi = wifi.lock().is_sta_enabled()?;
            if passed_throttle && connected_to_wifi {
                time_sms_last_sent = now;
                let twilio = (
                    dvc.sms_send_phone_number(),
                    dvc.sms_send_message_body(),
                    dvc.sms_send_twilio_phone_number(),
                    dvc.sms_send_twilio_account_sid(),
                    dvc.sms_send_twilio_auth_token(),
                );

                if let (Some(to), Some(body), Some(from), Some(sid), Some(auth_token)) = twilio {
                    if to.is_empty()
                        || body.is_empty()
                        || from.is_empty()
                        || sid.is_empty()
                        || auth_token.is_empty()
                    {
                        tracing::warn!("Some of the required fields are empty, skipping SMS");
                        continue;
                    }

                    tracing::info!("Sending SMS to {}", to);

                    let result = send_twilio_sms(to, body, from, sid, auth_token);

                    if let Err(e) = result {
                        tracing::error!("Error: {:?}", e);
                    }
                }
            }
        } else if ldr_photoresistor.is_low() && is_prev_high {
            is_prev_high = false;
            tracing::info!("Laser is in contact");
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

fn send_twilio_sms(
    to_phone_number: &str,
    body: &str,
    from_phone_number: &str,
    account_sid: &str,
    auth_token: &str,
) -> Result<()> {
    // #[derive(Serialize)]
    // #[serde(rename_all = "PascalCase")]
    // struct Params<'a> {
    //     to: &'a str,
    //     from: &'a str,
    //     body: &'a str,
    // }

    let url = format!(
        "https://@api.twilio.com/2010-04-01/Accounts/{sid}/Messages.json",
        sid = account_sid,
    );

    // let params = Params {
    //     to: to_phone_number,
    //     from: from_phone_number,
    //     body,
    // };
    let basic_auth_b64 =
        base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", account_sid, auth_token));

    let basic_auth_value = format!("Basic {}", basic_auth_b64);

    let basic_auth_header: (&str, &str) = ("Authorization", &basic_auth_value);

    let conn = EspHttpConnection::new(&client::Configuration::default())?;
    let mut http = HttpClient::wrap(conn);

    let headers = [
        basic_auth_header,
        ("Content-Type", "application/x-www-form-urlencoded"),
    ];
    let mut req = http.post(&url, &headers)?;

    // let params = serde_urlencoded::to_string(params)?;

    // manually generated URL-encoded string
    // for some reason it doesn't always work with serde_urlencoded
    write!(
        req,
        "To={}&From={}&Body={}",
        encode(to_phone_number),
        encode(from_phone_number),
        encode(body),
    )?;

    let mut res = req.submit()?;
    let status = res.status();
    if !(200..300).contains(&status) {
        let mut buf = [0u8; 2 * 1024];
        let read = res.read(&mut buf)?;

        let body = String::from_utf8_lossy(&buf[..read]);
        bail!("Status: {} {}", status, body);
    }

    Ok(())
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

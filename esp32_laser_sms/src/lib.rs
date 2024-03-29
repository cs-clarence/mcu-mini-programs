#![allow(dead_code)]
#![feature(decl_macro)]
#![feature(lazy_cell)]
#![feature(try_blocks)]

use core::{
    device::Device,
    wifi::{self, Wifi},
};

pub mod core;
pub mod service;
pub mod util;

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{peripherals::Peripherals, task::block_on},
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
use time::{
    format_description::FormatItem,
    macros::{format_description, offset},
    UtcOffset,
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

const HTTP_SERVER_STACK_SIZE: usize = 4 * 1024;

async fn async_main<'a>() -> Result<()> {
    tracing::info!("START: Memory [heap: {} free bytes]", get_free_heap_size(),);

    let peripherals = Peripherals::take()?;

    let storage = device_state::DeviceStateStorage::new("/spiflash/conf/device.bin");
    let cfg = device_state::DeviceStateManager::new_loaded_or_default(storage)?;
    let _dev_svc = device_state::DeviceStateService::new(cfg)?.into_send_sync();

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

    let _sntp = EspSntp::new(&SntpConf {
        sync_mode: SyncMode::Immediate,
        operating_mode: OperatingMode::Poll,
        servers: ["121.58.193.100"], // ntp.pagasa.dost.gov.ph
    })?;

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

    loop {
        delay_ms(100);
    }
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

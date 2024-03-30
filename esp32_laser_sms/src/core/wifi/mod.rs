use std::{borrow::BorrowMut, future::Future, str::FromStr};

use crate::util::{collection::alloc::hash::HashSet, result::error};
use derivative::Derivative;
use embedded_svc::{ipv4::IpInfo, wifi::AccessPointInfo};
use enumset::{enum_set, EnumSet};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    nvs::EspDefaultNvsPartition,
    timer::EspTaskTimerService,
    wifi::{self, AccessPointConfiguration, AsyncWifi, AuthMethod, ClientConfiguration, EspWifi},
};
use heapless::String as HString;
use serde::{Deserialize, Serialize};

use crate::util::{
    delay::non_blocking::delay_ms,
    sync::{arc_sync_mutex, ArcSyncMutex},
};

use super::persistent_state;
use crate::util::sync::IntoSendSync;

pub const DEFAULT_AP_SSID: &str = "Laser Security";
pub const DEFAULT_AP_PSK: &str = "";
pub const DEFAULT_AP_PROTOCOLS: EnumSet<wifi::Protocol> = enum_set!(wifi::Protocol::P802D11BGNLR);
pub const DEFAULT_AP_CHANNEL: u8 = 6;
pub const DEFAULT_AP_SECONDARY_CHANNEL: u8 = 11;
pub const DEFAULT_AP_MAX_CONNECTIONS: u16 = 1;

pub type WifiDriver<'a> = AsyncWifi<EspWifi<'a>>;
pub type ThreadSafeWifiDriver = ArcSyncMutex<WifiDriver<'static>>;
pub type SendSyncWifi<M = WifiStateManager> = ArcSyncMutex<Wifi<M>>;

#[derive(Debug, Deserialize, Serialize, Clone, Derivative)]
#[derivative(Hash, Eq, PartialEq)]
pub struct Credential {
    ssid: String,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    psk: String,
    bssid: [u8; 6],
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct WifiState {
    pub credentials: HashSet<Credential>,
}

pub type WifiStateStorage = persistent_state::BinaryFileStorage<WifiState>;

pub type WifiStateManager<S = WifiStateStorage> =
    persistent_state::PersistentStateManager<WifiState, S>;

pub type DefaultWifiStateManager = WifiStateManager<WifiStateStorage>;

pub type SendSyncWifiStateManager<S = WifiStateStorage> =
    persistent_state::SendSyncPersistentStateManager<WifiState, S>;

pub struct Wifi<M: BorrowMut<WifiStateManager> = WifiStateManager> {
    wifi: WifiDriver<'static>,
    config_manager: M,
}

pub type DefaultWifi = Wifi<DefaultWifiStateManager>;

impl<M: BorrowMut<WifiStateManager>> IntoSendSync for Wifi<M>
where
    SendSyncWifi<M>: Send + Sync,
{
    type SendSync = SendSyncWifi<M>;

    fn into_send_sync(self) -> Self::SendSync {
        arc_sync_mutex(self)
    }
}

pub type Result<T> = eyre::Result<T>;

fn auth_method(psk: &str) -> AuthMethod {
    if psk.is_empty() {
        AuthMethod::None
    } else {
        AuthMethod::WPAWPA2Personal
    }
}

fn send_future<R: Send + 'static>(
    f: impl Future<Output = R> + Send + 'static,
) -> impl Future<Output = R> + Send + 'static {
    f
}

impl<M: BorrowMut<WifiStateManager>> Wifi<M> {
    pub fn new(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        timer_service: EspTaskTimerService,
        nvs_partition: Option<EspDefaultNvsPartition>,
        config_manager: M,
    ) -> Result<Wifi<M>> {
        let wifi = Self::new_wifi(modem, sys_loop, timer_service, nvs_partition)?;

        Ok(Wifi {
            wifi,
            config_manager,
        })
    }

    fn new_wifi(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        timer_service: EspTaskTimerService,
        nvs_partition: Option<EspDefaultNvsPartition>,
    ) -> Result<WifiDriver<'static>> {
        let wifi = EspWifi::new(modem, sys_loop.clone(), nvs_partition)?;

        let wifi = AsyncWifi::wrap(wifi, sys_loop, timer_service)?;

        Ok(wifi)
    }

    pub fn into_wifi(self) -> WifiDriver<'static> {
        self.wifi
    }

    pub fn wifi(&self) -> &WifiDriver<'static> {
        &self.wifi
    }

    pub fn wifi_mut(&mut self) -> &mut WifiDriver<'static> {
        &mut self.wifi
    }

    pub async fn start(&mut self) -> Result<()> {
        if !self.is_started()? {
            self.wifi.start().await?;
        }

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if self.is_started()? {
            self.wifi.stop().await?;
        }

        Ok(())
    }

    pub fn is_connected(&self) -> Result<bool> {
        Ok(self.wifi.is_connected()?)
    }

    async fn internal_connect(&mut self) -> Result<()> {
        self.wifi.connect().await?;

        Ok(())
    }

    async fn restart(&mut self) -> Result<()> {
        let wifi = &mut self.wifi;

        wifi.stop().await?;

        wifi.start().await?;

        Ok(())
    }

    pub async fn start_ap(&mut self, ssid: &str, psk: &str) -> Result<()> {
        let auth_method = auth_method(psk);

        self.set_ap_configuration(AccessPointConfiguration {
            auth_method,
            password: HString::from_str(psk).map_err(|_| eyre::eyre!("Invalid password"))?,
            ssid: HString::from_str(ssid).map_err(|_| eyre::eyre!("Invalid SSID"))?,
            ssid_hidden: false,
            max_connections: DEFAULT_AP_MAX_CONNECTIONS,
            protocols: DEFAULT_AP_PROTOCOLS,
            channel: DEFAULT_AP_CHANNEL,
            secondary_channel: Some(DEFAULT_AP_SECONDARY_CHANNEL),
        })?;

        self.start().await?;

        Ok(())
    }

    pub async fn start_ap_default(&mut self) -> Result<()> {
        self.start_ap(DEFAULT_AP_SSID, DEFAULT_AP_PSK).await
    }

    pub fn has_saved_credentials(&mut self) -> bool {
        let cfg: &mut WifiStateManager = self.config_manager.borrow_mut();
        let cfg = cfg.state();

        !cfg.credentials.is_empty()
    }

    pub async fn reconnect(&mut self, retries: u8) -> Result<bool> {
        if !self.is_sta_enabled()? {
            // If the device is not in STA mode, we can't scan for APs
            self.set_client_configuration(ClientConfiguration::default())?;
        }

        let cfg: &mut WifiStateManager = self.config_manager.borrow_mut();
        let cfg = cfg.state();

        let creds = cfg.credentials.clone();

        let mut aps = self.scan_aps().await?;
        aps.sort_by(|a, b| a.signal_strength.cmp(&b.signal_strength));

        let filtered_creds = aps
            .into_iter()
            .filter_map(|ap| {
                creds
                    .iter()
                    .find(|cred| *cred.ssid == *ap.ssid && cred.bssid == ap.bssid)
                    .cloned()
            })
            .collect::<Vec<_>>();

        if filtered_creds.is_empty() {
            return Ok(false);
        }

        Ok({
            let mut did_connect = false;

            for cred in filtered_creds {
                let res = self
                    .connect(&cred.ssid, &cred.psk, None, None, retries)
                    .await;

                did_connect = res.is_ok();
            }

            if did_connect && self.is_ap_enabled()? {
                self.switch_to_sta_only().await?;
            }

            did_connect
        })
    }

    pub fn is_ap_enabled(&self) -> Result<bool> {
        Ok(self.wifi.wifi().driver().is_ap_enabled()?)
    }

    pub fn is_sta_enabled(&self) -> Result<bool> {
        Ok(self.wifi.wifi().driver().is_sta_enabled()?)
    }

    pub fn sta_ap_info(&mut self) -> Result<AccessPointInfo> {
        let info = self.wifi.wifi_mut().driver_mut().get_ap_info()?;

        Ok(info)
    }

    pub fn ap_ip_info(&self) -> Result<IpInfo> {
        let info = self.wifi.wifi().ap_netif();
        Ok(info.get_ip_info()?)
    }

    pub fn sta_ip_info(&self) -> Result<IpInfo> {
        let info = self.wifi.wifi().sta_netif();
        Ok(info.get_ip_info()?)
    }

    /// When the device is in STA+AP mode, this will switch it to STA only mode.
    /// Return true if the device was in STA+AP mode and is now in STA only mode or when the device was already in STA only mode.
    pub async fn switch_to_sta_only(&mut self) -> Result<bool> {
        let config = self.wifi.get_configuration()?;
        let client_config = match config {
            wifi::Configuration::Mixed(client_config, _) => client_config,
            wifi::Configuration::Client(_) => return Ok(true),
            _ => return Ok(false),
        };

        self.wifi
            .set_configuration(&wifi::Configuration::Client(client_config))?;

        self.start().await?;
        self.internal_connect().await?;

        Ok(true)
    }

    fn add_configuration(
        &mut self,
        client: Option<wifi::ClientConfiguration>,
        ap: Option<wifi::AccessPointConfiguration>,
    ) -> Result<()> {
        let current_config = self.wifi().get_configuration()?;

        let (prev_client, prev_ap) = match current_config {
            wifi::Configuration::None => (None, None),
            wifi::Configuration::Client(client) => (Some(client), None),
            wifi::Configuration::AccessPoint(ap) => (None, Some(ap)),
            wifi::Configuration::Mixed(client, ap) => (Some(client), Some(ap)),
        };

        let client = client.or(prev_client);
        let ap = ap.or(prev_ap);

        let config = match (client, ap) {
            (Some(client), Some(ap)) => wifi::Configuration::Mixed(client, ap),
            (Some(client), None) => wifi::Configuration::Client(client),
            (None, Some(ap)) => wifi::Configuration::AccessPoint(ap),
            (None, None) => wifi::Configuration::None,
        };

        self.wifi.set_configuration(&config)?;

        Ok(())
    }

    fn set_client_configuration(&mut self, client: wifi::ClientConfiguration) -> Result<()> {
        self.add_configuration(Some(client), None)
    }

    fn set_ap_configuration(&mut self, ap: wifi::AccessPointConfiguration) -> Result<()> {
        self.add_configuration(None, Some(ap))
    }

    /// When the device is in STA+AP mode, this will switch it to AP only mode.
    /// Return true if the device was in STA+AP mode and is now in AP only mode or when the device was already in AP only mode.
    pub async fn switch_to_ap_only(&mut self) -> Result<bool> {
        let config = self.wifi.get_configuration()?;
        let ap_config = match config {
            wifi::Configuration::Mixed(_, ap_config) => ap_config,
            wifi::Configuration::AccessPoint(_) => return Ok(true),
            _ => return Ok(false),
        };

        self.wifi
            .set_configuration(&wifi::Configuration::AccessPoint(ap_config))?;

        self.start().await?;

        Ok(true)
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn connect(
        &mut self,
        ssid: &str,
        psk: &str,
        bssid: Option<[u8; 6]>,
        channel: Option<u8>,
        retries: u8,
    ) -> Result<()> {
        let auth_method = auth_method(psk);

        let client = ClientConfiguration {
            auth_method,
            ssid: HString::from_str(ssid).map_err(|_| error!("Invalid SSID"))?,
            password: HString::from_str(psk).map_err(|_| error!("Invalid PSK"))?,
            bssid,
            channel,
        };
        self.set_client_configuration(client)?;
        self.start().await?;

        for i in 0..retries + 1 {
            let res = self.internal_connect().await;

            if res.is_ok() {
                self.wait_netif_up().await?;
                break;
            } else if i == retries {
                res?;
            }
            delay_ms(1000).await;
        }

        Ok(())
    }

    pub fn save_ap_credential(&mut self, ssid: &str, psk: &str, bssid: [u8; 6]) -> Result<()> {
        let cfg_mng: &mut WifiStateManager = self.config_manager.borrow_mut();
        let cred = Credential {
            ssid: ssid.to_string(),
            psk: psk.to_string(),
            bssid,
        };
        cfg_mng.update_state(move |cfg| {
            let mut cfg = cfg.clone();
            cfg.credentials.replace(cred);

            cfg
        })?;

        Ok(())
    }

    pub fn forget_ap(&mut self, ssid: &str, bssid: Option<[u8; 6]>) -> Result<()> {
        let cfg_mng: &mut WifiStateManager = self.config_manager.borrow_mut();
        cfg_mng.update_state(move |cfg| {
            let mut cfg = cfg.clone();
            cfg.credentials.retain(|cred| {
                if let Some(bssid) = bssid {
                    cred.ssid != ssid && cred.bssid != bssid
                } else {
                    cred.ssid != ssid
                }
            });
            cfg
        })?;

        Ok(())
    }

    pub fn is_up(&mut self) -> Result<bool> {
        Ok(self.wifi.is_up()?)
    }

    pub fn is_started(&mut self) -> Result<bool> {
        Ok(self.wifi.is_started()?)
    }

    pub async fn scan_aps(&mut self) -> Result<Vec<AccessPointInfo>> {
        self.start().await?;

        if !self.is_sta_enabled()? {
            self.set_client_configuration(ClientConfiguration::default())?;
        }

        let wifi = &mut self.wifi;

        let ap = wifi.scan().await?;

        Ok(ap)
    }

    pub fn saved_credentials(&mut self) -> Result<Vec<Credential>> {
        let cfg: &mut WifiStateManager = self.config_manager.borrow_mut();
        let cfg = cfg.state();

        let creds = cfg.credentials.clone();

        Ok(creds.into_iter().collect())
    }

    pub async fn wait_netif_up(&mut self) -> Result<()> {
        self.wifi.wait_netif_up().await?;

        Ok(())
    }
}

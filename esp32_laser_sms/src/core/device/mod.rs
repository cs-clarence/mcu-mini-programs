use std::fs;
use std::path::Path;
use std::ptr::addr_of_mut;

use esp_idf_svc::sys;

use crate::util::{ffi::esp::esp_unsafe, result::Result};

pub struct Device;

static mut WL_HANDLE: i32 = sys::WL_INVALID_HANDLE;

impl Device {
    pub fn init() -> Result<()> {
        esp_unsafe!(sys::esp_vfs_fat_spiflash_mount_rw_wl(
            c"/spiflash".as_ptr(),
            c"fs".as_ptr(),
            &sys::esp_vfs_fat_mount_config_t {
                format_if_mount_failed: true,
                max_files: 5,
                allocation_unit_size: sys::CONFIG_WL_SECTOR_SIZE as usize,
                disk_status_check_enable: false,
            },
            addr_of_mut!(WL_HANDLE),
        ))?;

        if !Path::new("/spiflash/conf").exists() {
            fs::create_dir_all("/spiflash/conf")?;
        }

        if !Path::new("/spiflash/data").exists() {
            fs::create_dir_all("/spiflash/data")?;
        }

        Ok(())
    }

    pub fn deinit() -> Result<()> {
        esp_unsafe!(sys::esp_vfs_fat_spiflash_unmount_rw_wl(
            c"/spiflash".as_ptr(),
            WL_HANDLE,
        ))?;

        Ok(())
    }

    pub fn restart() {
        unsafe { sys::esp_restart() };
    }

    pub fn reset() -> Result<()> {
        esp_unsafe!(sys::esp_vfs_fat_spiflash_format_rw_wl(
            c"/spiflash".as_ptr(),
            c"fs".as_ptr(),
        ))?;
        Self::restart();

        Ok(())
    }
}

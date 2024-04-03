#![feature(decl_macro)]

use esp_idf_svc::hal::{delay::TickType, gpio::PinDriver, i2c, peripherals::Peripherals};
use util::{result, tracing};

mod util;

pub fn main() -> result::Result<()> {
    esp_idf_svc::sys::link_patches();
    tracing::init()?;

    let p = Peripherals::take()?;
    let sda_pin = p.pins.gpio32;
    let scl_pin = p.pins.gpio33;
    let slave_addr = 0x04;
    let i2c = p.i2c0;
    let mut i2c_slave = i2c::I2cSlaveDriver::new(
        i2c,
        sda_pin,
        scl_pin,
        slave_addr,
        &i2c::config::SlaveConfig {
            rx_buf_len: 128,
            tx_buf_len: 128,
            ..Default::default()
        },
    )?;

    let solenoid_pin = p.pins.gpio25;
    let mut solenoid_lock = PinDriver::output(solenoid_pin)?;

    let indicator_led_pin = p.pins.gpio26;
    let mut indicator_led = PinDriver::output(indicator_led_pin)?;

    solenoid_lock.set_low()?;
    indicator_led.set_low()?;

    loop {
        let mut buff = [0u8; 1];
        let timeout = TickType(1000);
        let res = i2c_slave.read(&mut buff, timeout.into());

        if let Ok(read) = res {
            if read == 1 {
                let value = buff[0];
                tracing::info!("Value: {}", value);

                if value == 1 {
                    solenoid_lock.set_high()?;
                    indicator_led.set_high()?;
                } else {
                    solenoid_lock.set_low()?;
                    indicator_led.set_low()?;
                }
            }
        }
    }
}

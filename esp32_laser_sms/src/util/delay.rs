pub mod non_blocking {
    use embedded_hal::delay::DelayNs;
    use esp_idf_svc::hal::delay::FreeRtos;

    pub async fn delay_ms(ms: u64) {
        FreeRtos.delay_ms(ms as u32);
    }

    pub async fn delay_us(us: u64) {
        FreeRtos.delay_us(us as u32);
    }

    pub async fn delay_ns(ns: u64) {
        FreeRtos.delay_ns(ns as u32);
    }
}

pub mod blocking {
    use embedded_hal::delay::DelayNs;
    use esp_idf_svc::hal::delay::FreeRtos;

    pub fn delay_ms(ms: u64) {
        FreeRtos.delay_ms(ms as u32);
    }

    pub fn delay_us(us: u64) {
        FreeRtos.delay_us(us as u32);
    }

    pub fn delay_ns(ns: u64) {
        FreeRtos.delay_ns(ns as u32);
    }
}

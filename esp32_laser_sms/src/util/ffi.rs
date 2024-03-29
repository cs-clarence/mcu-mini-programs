pub mod esp {
    pub macro esp_unsafe($expr:expr) {
        esp_idf_svc::sys::esp!(unsafe { $expr })
    }

    pub macro esp($expr:expr) {
    esp_idf_svc::sys::esp!!{ $expr }
  }
}

pub macro cstr_ptr($expr:expr) {
    cstr::cstr!($expr).as_ptr()
}

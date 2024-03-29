pub mod ffi {
  use std::ffi::{CStr, FromBytesWithNulError};

  pub fn to_c_str(bytes: &[u8]) -> Result<&CStr, FromBytesWithNulError> {
    CStr::from_bytes_with_nul(bytes)
  }

  pub fn to_c_chars(
    bytes: &[u8],
  ) -> Result<*const std::ffi::c_char, FromBytesWithNulError> {
    Ok(to_c_str(bytes)?.as_ptr())
  }
}

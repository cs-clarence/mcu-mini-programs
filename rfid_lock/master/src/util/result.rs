pub macro error($($arg:tt)*) {
    eyre::eyre!($($arg)*)
}

pub type Result<T> = eyre::Result<T>;

pub type Error = eyre::Report;

pub macro bail($($arg:tt)*) {
  eyre::bail!($($arg)*)
}

#[allow(non_snake_case)]
#[inline(always)]
pub fn Ok<T>(value: T) -> Result<T> {
    std::result::Result::Ok(value)
}

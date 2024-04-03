use super::result;

pub use tracing::*;
use tracing_error::ErrorLayer;
use tracing_subscriber::layer::SubscriberExt;

pub fn init() -> result::Result<()> {
    // Bind the log crate to the ESP Logging facilitiegs
    let sub = tracing_subscriber::fmt()
        .finish()
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(sub)?;

    Ok(())
}

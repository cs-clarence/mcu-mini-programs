#![feature(decl_macro)]
use util::{result, tracing};

pub mod util;

fn run() -> result::Result<()> {
    Ok(())
}

pub fn main() -> eyre::Result<()> {
    esp_idf_svc::sys::link_patches();
    tracing::init()?;

    run()
}

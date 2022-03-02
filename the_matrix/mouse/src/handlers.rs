use crate::error::Result;
use crate::log::*;

pub fn setup_ctrlc_handler() -> Result<()> {
    let mut count = 0;
    Ok(ctrlc::set_handler(move || {
        count += 1;
        if count == 2 {
            std::process::exit(1);
        }
        warn!("Received SIGINT, send again to terminate.");
    })?)
}

mod run;
mod tests;
use std::process;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::tests::*;
use log::{error, info};
use once_cell::sync::Lazy;

static MANUAL: Lazy<bool> = Lazy::new(|| {
    if let Ok(value) = std::env::var("SALVATION_TESTS_MODE") {
        match value.as_str() {
            "manual" => true,
            "auto" => false,
            _default => {
                info!("SALVATION_TESTS_MODE environment value should be set to 'manual' or 'auto', it is now set to '{}'", value);
                false
            }
        }
    } else {
        false
    }
});

static SNAPSHOT_FAIL_COUNT: AtomicUsize = AtomicUsize::new(0);

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    env_logger::init();

    first_test().expect("Tests suit failed!");

    if SNAPSHOT_FAIL_COUNT.load(Ordering::SeqCst) == 0 {
        info!("RESULT: all tests passed.");
    } else {
        error!(
            "RESULT: {:?} of the snapshots tests failed, action required.",
            SNAPSHOT_FAIL_COUNT
        );
        process::exit(1);
    }
}

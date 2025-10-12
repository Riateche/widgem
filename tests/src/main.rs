mod button_tests;
mod label_tests;
mod menu_tests;
mod scroll_area_tests;
mod scroll_bar_tests;
mod simple_form;
mod text_input_tests;

use std::{
    env,
    path::{Path, PathBuf},
};

fn repo_dir() -> PathBuf {
    if let Ok(var) = env::var("WIDGEM_REPO_DIR") {
        PathBuf::from(var)
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("failed to get path parent")
            .into()
    }
}

fn main() -> anyhow::Result<()> {
    widgem_tester::run(repo_dir().join("tests/snapshots"))
}

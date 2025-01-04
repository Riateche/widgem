mod scroll_bar;
mod text_input;

use std::{
    env,
    path::{Path, PathBuf},
};

fn repo_dir() -> PathBuf {
    if let Ok(var) = env::var("SALVATION_REPO_DIR") {
        PathBuf::from(var)
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("failed to get path parent")
            .into()
    }
}

fn main() -> anyhow::Result<()> {
    salvation_test_kit::run(repo_dir().join("tests/snapshots"))
}

mod logic;
mod ui;

use {
    crate::{
        logic::{query_data, TesterLogic},
        ui::TesterUi,
    },
    anyhow::{bail, ensure, Context},
    clap::Parser,
    std::{
        path::PathBuf,
        process::{Command, Stdio},
    },
    widgem::Widget,
    widgem_tester::QueryAllResponse,
};

#[derive(Parser)]
pub struct Args {
    // TODO: allow specifying binary name, build mode (debug/release)
    /// Path to the tests crate.
    pub path: PathBuf,
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Sanity checks
    if !args.path.try_exists()? {
        bail!("no such directory: {:?}", args.path);
    }
    if !args.path.is_dir() {
        bail!("not a directory: {:?}", args.path);
    }
    let status = Command::new("cargo")
        .arg("--version")
        .status()
        .context("failed to run cargo")?;
    ensure!(status.success(), "failed to run cargo");

    let data = query_data(&args.path)?;
    let mut reviewer = TesterLogic::new(data.test_cases, args.path, data.snapshots_dir);
    if !reviewer.go_to_next_unconfirmed_file() {
        reviewer.go_to_test_case(0);
    }
    widgem::run(move |w| {
        w.base_mut().add_child::<TesterUi>(reviewer);
        Ok(())
    })
}

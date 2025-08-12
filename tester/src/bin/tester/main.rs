mod logic;
mod ui;

use {
    crate::{logic::Reviewer, ui::ReviewWidget},
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

    let output = Command::new("cargo")
        .args(["run", "--", "query", "all"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .current_dir(&args.path)
        .output()?;
    ensure!(output.status.success(), "failed to run cargo");
    let data = serde_json::from_slice::<QueryAllResponse>(&output.stdout).with_context(|| {
        format!(
            "couldn't parse output: {:?}",
            String::from_utf8_lossy(&output.stdout)
        )
    })?;

    let mut reviewer = Reviewer::new(data.test_cases, &data.snapshots_dir);
    if !reviewer.go_to_next_unconfirmed_file() {
        reviewer.go_to_test_case(0);
    }
    widgem::run(move |w| {
        w.base_mut().add_child::<ReviewWidget>(reviewer);
        Ok(())
    })
}

mod data;
mod logic;
mod ui;

use {
    crate::{data::Config, logic::TesterLogic, ui::TesterUi},
    anyhow::{bail, ensure, Context},
    clap::Parser,
    std::{path::PathBuf, process::Command},
    tracing_subscriber::{filter::LevelFilter, EnvFilter},
    widgem::Widget,
};

#[derive(Parser)]
pub struct Args {
    // TODO: allow specifying binary name, build mode (debug/release)
    /// Path to the tests crate.
    pub path: PathBuf,
    #[clap(long)]
    pub run_script: Option<PathBuf>,
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()?,
        )
        .init();

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

    let reviewer = TesterLogic::new(Config {
        tests_dir: args.path,
        run_script: args.run_script,
    })?;
    widgem::run(move |w| {
        w.base_mut().add_child::<TesterUi>(reviewer);
        Ok(())
    })
}

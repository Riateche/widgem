use std::{
    env,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{bail, Context as _};
use clap::Parser;
use context::{Context, SnapshotMode};
use salvation::App;
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};
use uitest::Connection;

pub mod context;
mod test_cases;

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

fn assets_dir() -> PathBuf {
    repo_dir().join("tests/assets")
}

fn test_cases_dir() -> PathBuf {
    repo_dir().join("tests/src/test_cases")
}

fn init_test_app() -> App {
    let fonts_path = assets_dir().join("fonts");
    App::new()
        .with_scale(1.0)
        .with_system_fonts(false)
        .with_font(fonts_path.join("NotoSans-Regular.ttf"))
        .with_font(fonts_path.join("NotoColorEmoji.ttf"))
        .with_font(fonts_path.join("NotoSansHebrew-VariableFont_wdth,wght.ttf"))
}

fn run_test_case(test_case: TestCase) -> anyhow::Result<()> {
    match test_case {
        TestCase::text_input => test_cases::text_input::run(),
    }
}

fn run_test_check(
    test_case: TestCase,
    connection: &mut Connection,
    mode: SnapshotMode,
    pid: u32,
    child: Child,
) -> anyhow::Result<Vec<String>> {
    let mut ctx = Context::new(
        connection,
        test_cases_dir().join(format!("{:?}", test_case)),
        mode,
        pid,
    )?;
    match test_case {
        TestCase::text_input => test_cases::text_input::check(&mut ctx)?,
    }
    verify_test_exit(child)?;
    Ok(ctx.finish())
}

fn verify_test_exit(mut child: Child) -> anyhow::Result<()> {
    const SINGLE_WAIT_DURATION: Duration = Duration::from_millis(200);
    const TOTAL_WAIT_DURATION: Duration = Duration::from_secs(5);

    let started = Instant::now();
    while started.elapsed() < TOTAL_WAIT_DURATION {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                bail!("test exited with status: {:?}", status);
            }
            return Ok(());
        }
        sleep(SINGLE_WAIT_DURATION);
    }
    println!(
        "test with pid={} hasn't exited (waited for {:?})",
        child.id(),
        TOTAL_WAIT_DURATION
    );
    child.kill()?;
    Ok(())
}

#[derive(Debug, Clone, Copy, EnumString, EnumIter, IntoStaticStr)]
#[allow(non_camel_case_types)]
enum TestCase {
    text_input,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
enum Args {
    Test {
        filter: Option<String>,
        #[clap(long)]
        check: bool,
    },
    Run {
        test_case: TestCase,
    },
    Approve {
        screenshot_path: String,
    },
}

fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    env_logger::init();
    let args = Args::parse();
    match args {
        // TODO: implement filter
        Args::Test { check, .. } => {
            let exe = env::args()
                .next()
                .context("failed to get current executable path")?;
            let mut conn = Connection::new()?;
            let mut all_fails = Vec::new();
            let mut num_total = 0;
            let mut num_failed = 0;
            let mode = if check {
                SnapshotMode::Check
            } else {
                SnapshotMode::Update
            };
            for test_case in TestCase::iter() {
                let test_name: &'static str = test_case.into();
                println!("running test: {}", test_name);
                let child = Command::new(&exe).args(["run", test_name]).spawn()?;
                let pid = child.id();
                let fails =
                    run_test_check(test_case, &mut conn, mode, pid, child).unwrap_or_else(|err| {
                        let fail = format!("test {:?} failed: {:?}", test_case, err);
                        println!("{fail}");
                        vec![fail]
                    });
                num_total += 1;
                if !fails.is_empty() {
                    num_failed += 1;
                }
                all_fails.extend(fails);
            }
            println!("-----------");
            println!("total tests: {}, failed tests: {}", num_total, num_failed);
            if !all_fails.is_empty() {
                println!("found issues:\n");
                for fail in all_fails {
                    println!("{fail}");
                }
                std::process::exit(1);
            }
        }
        Args::Run { test_case } => {
            run_test_case(test_case)?;
        }
        Args::Approve { screenshot_path } => {
            let Some(str) = screenshot_path.strip_suffix(".new.png") else {
                bail!(
                    "expected a path that ends with \".new.png\", got {:?}",
                    screenshot_path
                );
            };
            fs_err::rename(&screenshot_path, format!("{str}.png"))?;
            println!("Approved.");
        }
    }

    Ok(())
}

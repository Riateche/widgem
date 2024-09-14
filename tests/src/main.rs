use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{bail, Context as _};
use clap::Parser;
use context::{Context, SnapshotMode};
use fs_err::read_dir;
use review::{ReviewWidget, Reviewer};
use salvation::{widgets::WidgetExt, App};
use strum::IntoEnumIterator;
use test_cases::{run_test_case, run_test_check, TestCase};
use uitest::Connection;

pub mod context;
mod review;
mod test_cases;

// TODO: lazy
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

fn snapshots_dir() -> PathBuf {
    repo_dir().join("tests/snapshots")
}

fn test_app(default_scale: bool) -> App {
    let fonts_path = assets_dir().join("fonts");
    let mut app = App::new()
        .with_system_fonts(false)
        .with_font(fonts_path.join("NotoSans-Regular.ttf"))
        .with_font(fonts_path.join("NotoColorEmoji.ttf"))
        .with_font(fonts_path.join("NotoSansHebrew-VariableFont_wdth,wght.ttf"))
        .with_auto_repeat_delay(Duration::from_secs(2))
        .with_auto_repeat_interval(Duration::from_secs(1));
    if !default_scale {
        app = app.with_scale(1.0);
    }
    app
}

fn run_test_check_and_verify(
    test_case: TestCase,
    connection: &mut Connection,
    mode: SnapshotMode,
    pid: u32,
    child: Child,
) -> anyhow::Result<Vec<String>> {
    let mut ctx = Context::new(
        connection,
        snapshots_dir().join(format!("{:?}", test_case)),
        mode,
        pid,
    )?;
    run_test_check(&mut ctx, test_case)?;
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
        #[clap(long)]
        default_scale: bool,
    },
    Review,
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
        Args::Test { check, filter } => {
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
                let matches_filter = filter
                    .as_ref()
                    .map_or(true, |filter| test_name.contains(filter));
                if !matches_filter {
                    continue;
                }
                println!("running test: {}", test_name);
                let child = Command::new(&exe).args(["run", test_name]).spawn()?;
                let pid = child.id();
                let fails = run_test_check_and_verify(test_case, &mut conn, mode, pid, child)
                    .unwrap_or_else(|err| {
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
        Args::Run {
            test_case,
            default_scale,
        } => {
            let app = test_app(default_scale);
            run_test_case(app, test_case)?;
        }
        Args::Review => {
            let reviewer = Reviewer::new(&snapshots_dir());
            if reviewer.has_current_files() {
                salvation::run(|| ReviewWidget::new(reviewer).boxed())?;
            } else {
                println!("No unconfirmed snapshots found.");
            }
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

#[derive(Debug, Default)]
struct SingleSnapshotFiles {
    confirmed: Option<String>,
    unconfirmed: Option<String>,
}

fn discover_snapshots(test_case_dir: &Path) -> anyhow::Result<BTreeMap<u32, SingleSnapshotFiles>> {
    let mut unverified_files = BTreeMap::<u32, SingleSnapshotFiles>::new();
    for entry in read_dir(test_case_dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .to_str()
            .with_context(|| format!("non-unicode file name in test case dir: {:?}", entry.path()))?
            .to_string();
        if !name.ends_with(".png") {
            continue;
        }
        let mut iter = name.splitn(2, " - ");
        let first = iter.next().expect("never fails");
        iter.next()
            .with_context(|| format!("invalid snapshot name: {:?}", entry.path()))?;
        let step: u32 = first
            .parse()
            .with_context(|| format!("invalid snapshot name: {:?}", entry.path()))?;
        let files = unverified_files.entry(step).or_default();
        if name.ends_with(".new.png") {
            if let Some(unconfirmed) = &files.unconfirmed {
                bail!(
                    "duplicate unconfirmed files: {:?}, {:?}",
                    test_case_dir.join(unconfirmed),
                    entry.path()
                );
            }
            files.unconfirmed = Some(name);
        } else {
            if let Some(confirmed) = &files.confirmed {
                bail!(
                    "duplicate confirmed files: {:?}, {:?}",
                    test_case_dir.join(confirmed),
                    entry.path()
                );
            }
            files.confirmed = Some(name);
        }
    }
    Ok(unverified_files)
}

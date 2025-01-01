use {
    anyhow::{bail, Context as _},
    clap::Parser,
    context::{CheckContext, Context, SnapshotMode},
    fs_err::read_dir,
    review::{ReviewWidget, Reviewer},
    salvation::{widgets::WidgetExt, App},
    std::{
        collections::BTreeMap,
        env,
        path::{Path, PathBuf},
        process::Child,
        thread::sleep,
        time::{Duration, Instant},
    },
    strum::IntoEnumIterator,
    test_cases::{run_test_check, TestCase},
    uitest::Connection,
};

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
    ctx: &mut Context,
) -> anyhow::Result<Vec<String>> {
    run_test_check(ctx, test_case)?;
    if let Some(child) = ctx.as_check().child.take() {
        verify_test_exit(child)?;
    }
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
            let exe_path = env::args_os()
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
                conn.mouse_move_global(1, 1)?;
                println!("running test: {}", test_name);
                let mut ctx = Context::Check(CheckContext::new(
                    conn,
                    test_name.into(),
                    snapshots_dir().join(format!("{:?}", test_case)),
                    mode,
                    exe_path.clone(),
                )?);
                let fails = run_test_check_and_verify(test_case, &mut ctx).unwrap_or_else(|err| {
                    let fail = format!("test {:?} failed: {:?}", test_case, err);
                    println!("{fail}");
                    vec![fail]
                });
                conn = match ctx {
                    Context::Check(ctx) => ctx.connection,
                    Context::Run(_) => unreachable!(),
                };
                num_total += 1;
                if !fails.is_empty() {
                    num_failed += 1;
                }
                all_fails.extend(fails);
            }
            println!("-----------");
            println!("total tests: {}", num_total);
            if num_failed > 0 {
                println!("failed tests: {}", num_failed);
            } else {
                println!("all tests succeeded");
            }
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
            let mut ctx = Context::Run(Some(app));
            run_test_check(&mut ctx, test_case)?;
        }
        Args::Review => {
            let mut reviewer = Reviewer::new(&snapshots_dir());
            if !reviewer.go_to_next_unconfirmed_file() {
                reviewer.go_to_test_case(0);
            }
            salvation::run(|| ReviewWidget::new(reviewer).boxed())?;
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

#[derive(Debug)]
struct SingleSnapshotFile {
    pub full_name: String,
    pub description: String,
}

#[derive(Debug, Default)]
struct SingleSnapshotFiles {
    confirmed: Option<SingleSnapshotFile>,
    unconfirmed: Option<SingleSnapshotFile>,
}

fn discover_snapshots(test_case_dir: &Path) -> anyhow::Result<BTreeMap<u32, SingleSnapshotFiles>> {
    if !test_case_dir.try_exists()? {
        return Ok(BTreeMap::new());
    }
    let mut unverified_files = BTreeMap::<u32, SingleSnapshotFiles>::new();
    for entry in read_dir(test_case_dir)? {
        let entry = entry?;
        let full_name = entry
            .file_name()
            .to_str()
            .with_context(|| format!("non-unicode file name in test case dir: {:?}", entry.path()))?
            .to_string();
        let Some(name_without_png) = full_name.strip_suffix(".png") else {
            continue;
        };
        let mut iter = name_without_png.splitn(2, " - ");
        let first = iter.next().expect("never fails");
        let name_without_png_and_step = iter
            .next()
            .with_context(|| format!("invalid snapshot name: {:?}", entry.path()))?;
        let step: u32 = first
            .parse()
            .with_context(|| format!("invalid snapshot name: {:?}", entry.path()))?;
        let files = unverified_files.entry(step).or_default();
        if let Some(description) = name_without_png_and_step.strip_suffix(".new") {
            if let Some(unconfirmed) = &files.unconfirmed {
                bail!(
                    "duplicate unconfirmed files: {:?}, {:?}",
                    test_case_dir.join(&unconfirmed.full_name),
                    entry.path()
                );
            }
            files.unconfirmed = Some(SingleSnapshotFile {
                description: description.into(),
                full_name,
            });
        } else {
            if let Some(confirmed) = &files.confirmed {
                bail!(
                    "duplicate confirmed files: {:?}, {:?}",
                    test_case_dir.join(&confirmed.full_name),
                    entry.path()
                );
            }
            files.confirmed = Some(SingleSnapshotFile {
                description: name_without_png_and_step.into(),
                full_name,
            });
        }
    }
    Ok(unverified_files)
}

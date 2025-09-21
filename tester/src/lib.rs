pub mod context;

use {
    crate::context::SnapshotMode,
    anyhow::{bail, Context as _},
    clap::Parser,
    fs_err::read_dir,
    itertools::Itertools,
    serde::{Deserialize, Serialize},
    std::{
        collections::BTreeMap,
        env,
        path::{Path, PathBuf},
        process::{self, Child},
        sync::{Mutex, OnceLock},
        thread::sleep,
        time::{Duration, Instant, SystemTime},
    },
    tracing_subscriber::{filter::LevelFilter, EnvFilter},
    widgem::{App, AppBuilder},
};

#[doc(hidden)]
pub use ctor as __ctor;

pub use {
    crate::context::Context,
    uitest::{Button, Key, Window},
    widgem_macros::test,
};

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

#[derive(Default)]
pub struct Registry {
    #[allow(clippy::type_complexity)]
    tests: BTreeMap<String, Box<dyn FnOnce(&mut Context) -> anyhow::Result<()> + Send>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tests: BTreeMap::new(),
        }
    }

    pub fn tests(&self) -> impl Iterator<Item = &str> {
        self.tests.keys().map(|s| s.as_str())
    }

    pub fn has_test(&self, name: &str) -> bool {
        self.tests.contains_key(name)
    }

    pub fn add_test(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Context) -> anyhow::Result<()> + Send + 'static,
    ) {
        let old = self.tests.insert(name.into(), Box::new(f));
        assert!(old.is_none(), "duplicate test name");
    }

    pub fn run_test(&mut self, name: &str, ctx: &mut Context) -> anyhow::Result<()> {
        self.tests.remove(name).expect("invalid test name")(ctx)
    }
}

fn default_registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

pub fn add_test(name: &str, f: impl FnOnce(&mut Context) -> anyhow::Result<()> + Send + 'static) {
    default_registry().lock().unwrap().add_test(name, f);
}

// TODO: lazy
fn assets_dir() -> PathBuf {
    if let Ok(var) = env::var("WIDGEM_REPO_DIR") {
        PathBuf::from(var).join("tester/assets")
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
    }
}

fn test_app_builder(default_scale: bool) -> AppBuilder {
    let fonts_path = assets_dir().join("fonts");
    let mut app = App::builder()
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
    registry: &mut Registry,
    test_case: &str,
    ctx: &mut Context,
) -> anyhow::Result<Vec<String>> {
    registry.run_test(test_case, ctx)?;
    if let Some(child) = ctx.check(|c| c.take_child()) {
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

pub fn test_snapshots_dir(snapshots_dir: &Path, test_name: &str) -> PathBuf {
    snapshots_dir.join(test_name.split("::").join("/"))
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
        test_case: String,
        #[clap(long)]
        default_scale: bool,
    },
    Query {
        query: String,
    },
}

pub fn run(snapshots_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    let snapshots_dir = snapshots_dir.as_ref().to_owned();
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()?,
        )
        .init();

    let args = Args::parse();
    let mut registry = default_registry().lock().unwrap();

    match args {
        Args::Test { check, filter } => {
            let exe_path = env::args_os()
                .next()
                .context("failed to get current executable path")?;
            let uitest_context = uitest::Context::new()?;
            let mut all_fails = Vec::new();
            let mut num_total = 0;
            let mut num_failed = 0;
            let mode = if check {
                SnapshotMode::Check
            } else {
                SnapshotMode::Update
            };
            for test_name in registry.tests().map(|s| s.to_owned()).collect_vec() {
                let matches_filter = filter
                    .as_ref()
                    .is_none_or(|filter| test_name.contains(filter));
                if !matches_filter {
                    continue;
                }
                uitest_context.mouse_move_global(1, 1)?;
                println!("running test: {}", test_name);
                let mut ctx = Context::new_check(
                    uitest_context.clone(),
                    test_name.clone(),
                    test_snapshots_dir(&snapshots_dir, &test_name),
                    mode,
                    exe_path.clone(),
                )?;
                let fails = run_test_check_and_verify(&mut registry, &test_name, &mut ctx)
                    .unwrap_or_else(|err| {
                        let fail = format!("test {:?} failed: {:?}", test_name, err);
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
            if !registry.has_test(&test_case) {
                println!("Test not found! Available tests:");
                for name in registry.tests() {
                    println!("    {name}");
                }
                process::exit(1);
            }
            let app = test_app_builder(default_scale);
            let mut ctx = Context::new_run(app);
            registry.run_test(&test_case, &mut ctx)?;
        }
        Args::Query { query } => {
            if query != "all" {
                bail!("unknown query");
            }
            let data = QueryAllResponse {
                snapshots_dir,
                test_cases: registry.tests().map(|s| s.to_owned()).collect(),
            };
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryAllResponse {
    pub snapshots_dir: PathBuf,
    pub test_cases: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct SingleSnapshotFile {
    pub full_name: String,
    pub description: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}

#[derive(Debug, Default)]
pub struct SingleSnapshotFiles {
    pub confirmed: Option<SingleSnapshotFile>,
    pub unconfirmed: Option<SingleSnapshotFile>,
}

pub fn discover_snapshots(
    test_case_dir: &Path,
) -> anyhow::Result<BTreeMap<u32, SingleSnapshotFiles>> {
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
                path: entry.path(),
                modified: entry.metadata()?.modified()?,
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
                path: entry.path(),
                modified: entry.metadata()?.modified()?,
                full_name,
            });
        }
    }
    Ok(unverified_files)
}

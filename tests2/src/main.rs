use std::{
    env,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{bail, Context};
use clap::Parser;
use salvation::{event_loop::App, winit::error::EventLoopError};
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};
use uitest::Connection;

mod test_cases;

fn assets_dir() -> PathBuf {
    if let Ok(var) = env::var("SALVATION_TESTS_ASSETS_DIR") {
        PathBuf::from(var)
    } else {
        // TODO: update relative path
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/assets")
    }
}

fn init_test_app() -> App {
    // TODO: update relative path
    let fonts_path = assets_dir().join("fonts");
    App::new()
        .with_scale(1.0)
        .with_system_fonts(false)
        .with_font(fonts_path.join("NotoSans-Regular.ttf"))
        .with_font(fonts_path.join("NotoColorEmoji.ttf"))
        .with_font(fonts_path.join("NotoSansHebrew-VariableFont_wdth,wght.ttf"))
}

fn run_test_case(test_case: TestCase) -> Result<(), EventLoopError> {
    match test_case {
        TestCase::TextInput => test_cases::text_input::run(),
    }
}

fn run_test_check(test_case: TestCase, conn: &mut Connection, pid: u32) -> anyhow::Result<()> {
    match test_case {
        TestCase::TextInput => test_cases::text_input::check(conn, pid),
    }
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
enum TestCase {
    TextInput,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    filter: Option<String>,
}

const TEST_CASE_ENV_VAR: &str = "SALVATION_TESTS_TEST_CASE";

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("args: {:?}", args);
    if let Ok(test_case) = env::var(TEST_CASE_ENV_VAR) {
        run_test_case(test_case.parse()?)?;
    } else {
        // TODO: implement filter
        let exe = env::args()
            .next()
            .context("failed to get current executable path")?;
        let mut conn = Connection::new()?;
        for test_case in TestCase::iter() {
            let test_name: &'static str = test_case.into();
            println!("running test: {}", test_name);
            let child = Command::new(&exe)
                .env(TEST_CASE_ENV_VAR, test_name)
                .spawn()?;
            let pid = child.id();
            run_test_check(test_case, &mut conn, pid)?;
            verify_test_exit(child)?;
        }
    }
    Ok(())
}

// 2
/*
fn main_old() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;

    for (key, val) in std::env::vars_os() {
        println!("{:?}={:?}", key, val);
    }

    // fn normalized(filename: &str) -> String {
    //     filename
    //         .replace("|", "")
    //         .replace("\\", "")
    //         .replace(":", "")
    //         .replace("/", "")
    // }

    let start = Instant::now();
    let c = Connection::new()?;
    let windows = c.all_windows().unwrap();

    //let mut i = 0;

    let active = c.active_window_id()?;

    for window in windows {
        // if window.is_minimized() {
        //     continue;
        // }

        println!(
            "Window: {:?} pid={:?} {:?} {:?} {:?}",
            window.id(),
            window.pid(),
            window.title(),
            (window.x(), window.y(), window.width(), window.height()),
            (window.is_minimized(), window.is_maximized())
        );
        if window.id() == active {
            println!("active!");
        }
        if window.title().contains("Geany") {
            println!("activate!");
            window.activate()?;
            // window.mouse_move(20, 40)?;
            // c.mouse_click(1)?;
            window.close()?;
        }

        // let image = window.capture_image().unwrap();
        // image
        //     .save(format!(
        //         "/tmp/1/window-{}-{}.png",
        //         i,
        //         normalized(window.title())
        //     ))
        //     .unwrap();

        //i += 1;
    }

    println!("{:?}", start.elapsed());
    println!("sleeping");
    std::thread::sleep(std::time::Duration::from_secs(3));
    if std::env::args().nth(1).unwrap() == "e" {
        panic!("emulated error!");
    }
    Ok(())
}
*/

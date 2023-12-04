use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use anyhow::{anyhow, bail, Context, Result};
use salvation::event_loop::{App, CallbackContext, UserEvent};
use salvation::system::with_system;
use salvation::winit::event::WindowEvent;
use salvation::winit::event_loop::EventLoopProxy;
use tiny_skia::Pixmap;

pub struct TestContext {
    event_loop_proxy: EventLoopProxy<UserEvent>,
    snapshots_prefix: String,
    snapshot_dir: PathBuf,
    current_snapshot_index: usize,
}

use log::{error, info};

use crate::{MANUAL, SNAPSHOT_FAIL_COUNT};

impl TestContext {
    fn current_snapshot_prefix(&self) -> String {
        format!("{}{}", self.snapshots_prefix, self.current_snapshot_index)
    }

    fn quit(&self) {
        //TODO close all windows, not just one
        let _ = self.event(0, WindowEvent::CloseRequested);
    }

    pub fn snapshot(&mut self, description: &str) -> Result<()> {
        let (tx, rx) = mpsc::sync_channel(1);
        if let Err(e) = self
            .event_loop_proxy
            .send_event(UserEvent::SnapshotRequest(tx))
        {
            bail!("There has been an error sending snapshot event: {:?}", e);
        };
        let snapshot = rx
            .recv()
            .map_err(|e| anyhow!("There has been an error receiving snapshot: {:?}", e))?;

        let current_snapshot_prefix = self.current_snapshot_prefix();

        for (i, s) in snapshot.0.iter().enumerate() {
            let full_snapshot_path = self
                .snapshot_dir
                .join(format!("{}_{}.png", current_snapshot_prefix, i));
            let full_new_snapshot_path = self
                .snapshot_dir
                .join(format!("{}_{}_new.png", current_snapshot_prefix, i));
            let pixmap = Pixmap::load_png(&full_snapshot_path);
            match pixmap {
                Err(e) => {
                    if *MANUAL {
                        error!(
                            "Test failed: {:?}, description: {}, error: {:?}",
                            full_snapshot_path, description, e
                        );
                        SNAPSHOT_FAIL_COUNT.fetch_add(1, Ordering::SeqCst);
                        //probably just no old snapshot, let's try and write a new one
                        s.save_png(&full_new_snapshot_path)
                            .context(format!("{:?}", full_new_snapshot_path))?;
                        info!("Saved snapshot: {:?}", full_new_snapshot_path);
                    } else {
                        return Err(anyhow!(
                            "Test failed: {:?}, description: {}, error: {:?}",
                            full_snapshot_path,
                            description,
                            e
                        ));
                    }
                }
                Ok(old_snapshot) => {
                    if old_snapshot == *s {
                        if *MANUAL {
                            info!("Snapshot test passed: {:?}", full_snapshot_path);
                            if full_new_snapshot_path.try_exists()? {
                                fs_err::remove_file(&full_new_snapshot_path)?;
                            }
                        }
                    } else {
                        let error_text = format!(
                            "Test failed: {:?}, description: {}, error: snapshots do not match",
                            full_snapshot_path, description
                        );
                        if *MANUAL {
                            error!("{}", error_text);
                            SNAPSHOT_FAIL_COUNT.fetch_add(1, Ordering::SeqCst);
                            s.save_png(&full_new_snapshot_path)
                                .context(format!("{:?}", full_new_snapshot_path))?;
                            info!("Saved snapshot: {:?}", full_new_snapshot_path);
                        } else {
                            return Err(anyhow!(error_text));
                        }
                    }
                }
            }
        }

        self.current_snapshot_index += 1;
        Ok(())
    }

    pub fn event(&self, window_index: usize, event: WindowEvent) -> Result<()> {
        self.event_loop_proxy
            .send_event(UserEvent::DispatchWindowEvent(window_index, event))
            .map_err(|e| anyhow!("There has been an error sending event: {:?}", e))
    }
}

pub fn run_inner<State: 'static>(
    make_state: impl FnOnce(&mut CallbackContext<State>) -> State,
    run_test: impl FnOnce(&mut TestContext) -> Result<()> + Send + 'static,
    snapshots_prefix: String,
) -> Result<()> {
    let handle = Arc::new(Mutex::new(None));
    let make_state_with_tests = |callback_context: &mut CallbackContext<State>| {
        let state = make_state(callback_context);
        let event_loop_proxy = with_system(|system| system.event_loop_proxy.clone());
        let mut test_context = TestContext {
            event_loop_proxy,
            snapshots_prefix,
            snapshot_dir: Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/snapshots"),
            current_snapshot_index: 0,
        };
        let h = thread::spawn(move || {
            let result = run_test(&mut test_context);
            test_context.quit();
            result
        });
        let mut locked_handle = handle.lock().unwrap();
        *locked_handle = Some(h);
        state
    };
    let fonts_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("fonts");
    App::new()
        .with_system_fonts(false)
        .with_font(fonts_path.join("NotoSans-Regular.ttf"))
        .with_font(fonts_path.join("NotoColorEmoji.ttf"))
        .with_font(fonts_path.join("NotoSansHebrew-VariableFont_wdth,wght.ttf"))
        .run(make_state_with_tests)
        .map_err(|e| anyhow!("Error while running test event loop: {:?}", e))?;
    let mut locked_handle = handle.lock().unwrap();
    let handle = locked_handle.take();
    if let Some(handle) = handle {
        match handle.join() {
            Err(e) => Err(anyhow!("Failed to join handle: {:?}", e)),
            Ok(result) => result,
        }
    } else {
        Err(anyhow!("No handle"))
    }
}

macro_rules! run {
    ($a:expr,$b:expr) => {{
        use crate::run::run_inner;
        use itertools::Itertools;
        use std::iter;

        let path = module_path!();
        let mut split_path = path.split("::");
        let mut snapshot_prefix = if let Some(first_part) = split_path.next() {
            if first_part != env!("CARGO_CRATE_NAME") {
                iter::once(first_part).chain(split_path).join("_")
            } else {
                split_path.join("_")
            }
        } else {
            split_path.join("_")
        };
        snapshot_prefix.push('_');
        run_inner($a, $b, snapshot_prefix)
    }};
}

pub(crate) use run;

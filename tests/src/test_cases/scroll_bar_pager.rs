pub use super::scroll_bar::RootWidget;

use {
    crate::context::Context,
    std::{thread::sleep, time::Duration},
};

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let mut window = ctx.wait_for_window_by_pid()?;
    // Workaround for winit issue:
    // https://github.com/rust-windowing/winit/issues/2841
    window.minimize()?;
    window.activate()?;
    window.resize(160, 66)?;

    window.mouse_move(100, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;
    ctx.connection.mouse_click(1)?;
    ctx.snapshot(&mut window, "page right")?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&mut window, "no highlight")?;

    window.mouse_move(43, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;
    ctx.connection.mouse_click(1)?;
    ctx.snapshot(&mut window, "page left")?;
    window.mouse_move(0, 0)?;
    ctx.snapshot(&mut window, "no highlight")?;

    ctx.connection.key("1")?;
    ctx.snapshot(&mut window, "increase range")?;
    window.mouse_move(100, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;

    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "page right")?;
    sleep(Duration::from_millis(700)); // auto repeat delay is 2 s; snapshot delay is 0.5 s
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released pager - no auto repeat")?;

    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "page right")?;
    sleep(Duration::from_millis(1300));
    ctx.snapshot(&mut window, "page right - first auto repeat")?;
    sleep(Duration::from_millis(500));
    ctx.snapshot(&mut window, "page right - second auto repeat")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released pager - no more auto repeats")?;

    window.close()?;
    Ok(())
}

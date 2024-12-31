pub use super::scroll_bar::RootWidget;

use crate::context::Context;

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let mut window = ctx.wait_for_window_by_pid()?;
    // Workaround for winit issue:
    // https://github.com/rust-windowing/winit/issues/2841
    window.minimize()?;
    window.activate()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

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
    ctx.connection.mouse_up(1)?;
    ctx.changing_expected = false;
    ctx.snapshot(&mut window, "released pager - no auto repeat")?;
    ctx.changing_expected = true;

    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "page right")?;
    ctx.snapshot(&mut window, "page right - first auto repeat")?;
    ctx.snapshot(&mut window, "page right - second auto repeat")?;
    ctx.connection.mouse_up(1)?;
    ctx.changing_expected = false;
    ctx.snapshot(&mut window, "released pager - no more auto repeats")?;
    ctx.changing_expected = true;

    window.close()?;
    Ok(())
}

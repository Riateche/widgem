use crate::context::Context;

pub use super::scroll_bar::RootWidget;

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let mut window = ctx.wait_for_window_by_pid()?;
    // Workaround for winit issue:
    // https://github.com/rust-windowing/winit/issues/2841
    window.minimize()?;
    window.activate()?;
    window.resize(160, 66)?;

    window.mouse_move(60, 20)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(300, 24)?;
    ctx.snapshot(&mut window, "dragged all the way right")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider - no highlight")?;

    window.mouse_move(90, 24)?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(0, 20)?;
    ctx.snapshot(&mut window, "dragged all the way left")?;
    window.mouse_move(20, 20)?;
    ctx.snapshot(&mut window, "still all the way left")?;
    window.mouse_move(58, 20)?;
    ctx.snapshot(&mut window, "no longer all the way left")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider")?;

    window.close()?;
    Ok(())
}

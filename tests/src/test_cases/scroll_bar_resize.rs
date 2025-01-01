pub use super::scroll_bar::RootWidget;

use {crate::context::Context, salvation::widgets::WidgetExt};

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|| RootWidget::new().boxed())?;
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.resize(1, 1)?;
    ctx.snapshot(&mut window, "min size")?;

    window.resize(200, 66)?;
    ctx.snapshot(&mut window, "resized")?;

    window.resize(300, 66)?;
    ctx.snapshot(&mut window, "resized")?;

    window.resize(300, 200)?;
    ctx.snapshot(&mut window, "no change - fixed y size")?;

    window.resize(300, 5)?;
    ctx.snapshot(&mut window, "min y size")?;

    ctx.connection().key("r")?;
    ctx.snapshot(&mut window, "changed to vertical scroll bar")?;

    window.resize(1, 1)?;
    ctx.snapshot(&mut window, "min size")?;

    window.resize(200, 200)?;
    ctx.snapshot(&mut window, "resized")?;

    window.resize(200, 300)?;
    ctx.snapshot(&mut window, "resized")?;

    window.resize(1, 300)?;
    ctx.snapshot(&mut window, "min x size")?;

    window.close()?;
    Ok(())
}

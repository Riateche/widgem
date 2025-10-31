use {
    widgem::widgets::{TextArea, Window},
    widgem_tester::Context,
};

#[widgem_tester::test]
pub fn main(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|root| {
        let window = root.set_main_content(Window::init(module_path!().into()))?;
        window
            .set_main_content(TextArea::init())?
            .set_text("Hello world!\nBye world!");
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("label")?;
    window.close()?;
    Ok(())
}

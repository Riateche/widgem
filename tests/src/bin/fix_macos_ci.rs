use {
    image::RgbaImage,
    std::{thread::sleep, time::Duration},
};

fn main() -> anyhow::Result<()> {
    let ctx = uitest::Context::new()?;
    loop {
        println!("taking screenshot");
        let image = ctx.capture_full_screen()?;
        if let Some((x, y)) = find_button(&image) {
            println!("found button at ({x}, {y})");
            ctx.mouse_move_global(x as i32, y as i32)?;
            ctx.mouse_click(uitest::Button::Left)?;
            return Ok(());
        }
        sleep(Duration::from_millis(200));
    }
}

fn find_button(image: &RgbaImage) -> Option<(u32, u32)> {
    let target_color = [0x18, 0x88, 0xff, 0xff];

    let mut count = 0;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel
            .0
            .into_iter()
            .zip(target_color)
            .all(|(a, b)| a.abs_diff(b) < 3)
        {
            count += 1;
            if count == 90 {
                return Some((x, y));
            }
        } else {
            count = 0;
        }
    }
    None
}

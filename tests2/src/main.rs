use uitest::Connection;

// 2
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let c = Connection::new();
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

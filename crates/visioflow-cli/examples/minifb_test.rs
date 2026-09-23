use minifb::{Key, Window, WindowOptions};
use std::time::Instant;
use visioflow_cli::screen_bounds::ensure_interactive_desktop;

fn main() {
    const WIDTH: usize = 500;
    const HEIGHT: usize = 400;

    println!("==================================================");
    println!("     Minifb Window Interactive Desktop Test       ");
    println!("==================================================");

    // Call VisioFlow's official desktop binder
    ensure_interactive_desktop();
    println!("[OK] ensure_interactive_desktop() invoked.");

    let mut opts = WindowOptions::default();
    opts.topmost = true;

    let mut window = match Window::new(
        "VisioFlow Display Test - Press ESC to close",
        WIDTH,
        HEIGHT,
        opts,
    ) {
        Ok(win) => {
            println!("[OK] Window::new succeeded.");
            win
        }
        Err(err) => {
            eprintln!("[FAIL] Window::new failed: {err}");
            return;
        }
    };

    window.set_target_fps(60);

    let buffer: Vec<u32> = vec![0x002ECC71; WIDTH * HEIGHT]; // Emerald Green

    println!("\n>>> Floating Emerald Green window is now visible on your display. <<<");
    println!("Looping for up to 30 seconds (press ESC or close window to finish)...\n");

    let start = Instant::now();
    let mut frame_count = 0u64;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if start.elapsed().as_secs() >= 30 {
            break;
        }

        if let Err(e) = window.update_with_buffer(&buffer, WIDTH, HEIGHT) {
            eprintln!("[FAIL] window.update_with_buffer: {e}");
            break;
        }

        frame_count += 1;
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    println!("Diagnostic test completed successfully. Total frames rendered: {frame_count}");
}

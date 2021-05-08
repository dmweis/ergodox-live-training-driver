use anyhow::Result;
use ergodox_driver::driver;

fn main() -> Result<()> {
    let mut device = driver::ErgodoxDriver::connect_to_first()?;
    match device.keyboard_type() {
        driver::KeyboardType::Ergodox => println!("Detected Ergodox"),
        _ => println!(
            "Detected {:?} keyboard, only Ergodox fully supported",
            device.keyboard_type()
        ),
    }
    for _ in 0..10 {
        device.write(driver::Command::LandingPage)?;
        for message in device.read()? {
            if let driver::Event::LayoutName(ref layout_id) = message {
                println!(
                    "Layout id: {} revision: {}",
                    layout_id.id(),
                    layout_id.revision()
                );
                // TODO (David): The first path element may have to change for other boards
                // can't test it without them
                let link = format!(
                    "https://configure.zsa.io/ergodox-ez/layouts/{}/{}",
                    layout_id.id(),
                    layout_id.revision()
                );
                if webbrowser::open(&link).is_ok() {
                    println!("Opened {}", link);
                } else {
                    eprintln!("Failed to open {}", link);
                }
                return Ok(());
            }
        }
    }
    Ok(())
}

mod driver;
mod layout_store_client;

use anyhow::Result;
use log::*;
use simplelog::*;

fn main() -> Result<()> {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;
    let mut device = driver::ErgodoxDriver::connect_to_first()?;
    info!("Connected to {:?}", device.keyboard_type());
    info!("Querying layout");
    let mut layout: Option<layout_store_client::Layout> = None;
    loop {
        for message in device.read()? {
            if let driver::Event::LayoutName(ref layout_id) = message {
                layout = layout_store_client::query_layout(
                    layout_id.id().to_owned(),
                    layout_id.revision().to_owned(),
                )
                .ok();
                info!(
                    "Layout received id: {} revision: {}",
                    layout_id.id(),
                    layout_id.revision()
                );
                break;
            }
            info!("Received other message: {:?}", message)
        }
        if layout.is_some() {
            break;
        }
        device.write(driver::Command::LandingPage)?;
    }
    info!("Pairing, please press the Oryx key");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        device.write(driver::Command::Pair)?;
        let mut paired = false;
        for message in device.read()? {
            if let driver::Event::Paired = message {
                paired = true;
            }
            info!("{:?}", message);
        }
        if paired {
            device.write(driver::Command::LiveTraining)?;
            break;
        }
    }
    info!("Paired!");
    let mut current_layer_index = 0;
    loop {
        for event in device.read()? {
            match event {
                driver::Event::Layer(layer_index) => {
                    info!("Layer switched to {}", layer_index);
                    current_layer_index = layer_index;
                }
                driver::Event::KeyUp(key_code) | driver::Event::KeyDown(key_code) => {
                    if let Some(layout) = &layout {
                        let key = layout.get_key(key_code, current_layer_index as usize);
                        info!("Key {:#?}", key);
                    }
                }
                driver::Event::LiveTraining => info!("Started live training! Click some buttons!"),
                _ => info!("Other event {:?}", event),
            }
        }
    }
}

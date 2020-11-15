mod driver;
mod layout_store_client;

use anyhow::Result;
use log::*;
use simplelog::*;

fn main() -> Result<()> {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed)?;
    let mut device = driver::ErgodoxDriver::connect_to_first()?;
    device.write(driver::Command::LandingPage)?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut layout: Option<layout_store_client::Layout> = None;
    device.read()?.iter().for_each(|message| {
        if let driver::Event::LayoutName(ref layout_id) = message {
            layout =
                layout_store_client::query_layout(layout_id.id.clone(), layout_id.revision.clone())
                    .ok();
        }
        info!("{:?}", message)
    });
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
                _ => info!("Event {:?}", event),
            }
        }
    }
}

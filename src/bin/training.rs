use anyhow::Result;
use ergodox_driver::{driver, layout_store_client};
use log::*;
use simplelog::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn main() -> Result<()> {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_handle = keep_running.clone();

    ctrlc::set_handler(move || {
        keep_running_handle.store(false, Ordering::Release);
        info!("Caught interrupt");
    })?;

    let mut device = driver::ErgodoxDriver::connect_to_first()?;
    info!("Connected to {:?}", device.keyboard_type());
    info!("Querying layout");
    let mut layout: Option<layout_store_client::Layout> = None;
    while keep_running.load(Ordering::Acquire) {
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
    if keep_running.load(Ordering::Acquire) {
        if let Some(layout) = &layout {
            info!("Oryx keys are at:");
            for (key_position, layer) in layout.find_oryx_keys() {
                info!("   {} layer: {}", key_position, layer);
            }
        }
        info!("Pairing, please press the Oryx key");
    }
    while keep_running.load(Ordering::Acquire) {
        device.write(driver::Command::Pair)?;
        let mut paired = false;
        for message in device.read()? {
            if let driver::Event::Paired = message {
                paired = true;
                info!("Paired!");
            } else {
                info!("Received other message: {:?}", message);
            }
        }
        if paired {
            device.write(driver::Command::LiveTraining)?;
            break;
        }
    }
    let mut current_layer_index = 0;
    while keep_running.load(Ordering::Acquire) {
        for event in device.read()? {
            match event {
                driver::Event::Layer(layer_index) => {
                    info!("Layer switched to {}", layer_index);
                    current_layer_index = layer_index;
                }
                driver::Event::KeyUp(key_code) | driver::Event::KeyDown(key_code) => {
                    if let Some(layout) = &layout {
                        let key = layout.get_key(key_code, current_layer_index as usize);
                        if let Some(key) = key {
                            info!("pos {} info {}", key_code, key);
                        } else {
                            warn!("Unknown key {}", key_code);
                        }
                    }
                }
                driver::Event::LiveTraining => info!("Started live training! Click some buttons!"),
                _ => info!("Other event {:?}", event),
            }
        }
    }
    info!("Exiting...");
    Ok(())
}

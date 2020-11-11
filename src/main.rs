use anyhow::Result;
use log::*;
use rusb;
use simplelog::*;
use thiserror::Error;

/// legacy - zsa's vendor id
const VENDOR_IDS: [u16; 2] = [0xfeed, 0x3297];
/// legacy - standard - shine - glow
const ERGODOX_IDS: [u16; 4] = [0x1307, 0x4974, 0x4975, 0x4976];
/// legacy - standard - glow
const PLANCK_IDS: [u16; 3] = [0x6060, 0xc6ce, 0xc6cf];
/// mk1
const MOONLANDER_IDS: [u16; 1] = [0x1969];

#[derive(Debug, Error)]
#[non_exhaustive]
enum DriverError {
    #[error("Failed to iterate devices")]
    FailedToIterateDevices,
    #[error("Failed to get device description")]
    FailedToGetDescription,
    #[error("Device not found")]
    DeviceNotFound,
    #[error("Failed to open device")]
    FailedToOpen,
}

const CMD_PAIR: u8 = 0;
const CMD_LANDING_PAGE: u8 = 1;
const CMD_GET_LAYER: u8 = 2;
const CMD_LIVE_TRAINING: u8 = 3;
const EVT_PAIRED: u8 = 0;
const EVT_LAYER: u8 = 2;
const EVT_LIVE_TRAINING: u8 = 3;
const EVT_KEYDOWN: u8 = 17;
const EVT_KEYUP: u8 = 18;

const SEPARATOR: u8 = 254;

#[derive(Debug)]
struct Configuration {
    config_id: u8,
    iface_id: u8,
    in_endpoint_address: u8,
    out_endpoint_address: u8,
}

fn main() -> Result<()> {
    TermLogger::init(LevelFilter::Trace, Config::default(), TerminalMode::Mixed)?;
    let device = find_device()?.ok_or(DriverError::DeviceNotFound)?;
    open_device(device)?;
    Ok(())
}

fn open_device(device: rusb::Device<rusb::GlobalContext>) -> Result<()> {
    let config = select_interface(&device)?.ok_or(DriverError::FailedToOpen)?;
    info!("Selected config {:?}", &config);
    let mut device_handle = device.open()?;

    info!(
        "Has kernel driver {}",
        device_handle.kernel_driver_active(config.iface_id)?
    );

    let active_config = device_handle.active_configuration()?;
    if active_config != config.config_id {
        error!("Desired config not active");
        device_handle
            .set_active_configuration(config.config_id)
            .expect("Failed setting desired config");
    }
    device_handle.claim_interface(config.iface_id)?;

    write_read(&mut device_handle, &config, CMD_LANDING_PAGE)?;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let data = write_read(&mut device_handle, &config, CMD_PAIR)?;
        if data.len() == 1 && data.get(0).unwrap() == &EVT_PAIRED {
            break;
        }
    }
    // write_read(&mut device_handle, &config, CMD_LIVE_TRAINING);
    // write_read(&mut device_handle, &config, CMD_GET_LAYER);
    Ok(())
}

/// CMD_LANDING_PAGE
/// CMD_PAIR
/// CMD_LIVE_TRAINING
/// CMD_GET_LAYER

fn write_read(
    device_handle: &mut rusb::DeviceHandle<rusb::GlobalContext>,
    config: &Configuration,
    command: u8,
) -> Result<Vec<u8>> {
    let written = device_handle.write_interrupt(
        config.out_endpoint_address,
        &[command],
        std::time::Duration::from_millis(100),
    )?;
    info!("Written data {}", written);
    let mut buf = [0; 64];
    let read_size = device_handle
        .read_interrupt(
            config.in_endpoint_address,
            &mut buf,
            std::time::Duration::from_millis(1000),
        )
        .unwrap_or_default();
    info!("Read data {:?}", &buf[0..read_size]);
    Ok(buf.into())
}

fn select_interface(device: &rusb::Device<rusb::GlobalContext>) -> Result<Option<Configuration>> {
    let device_descriptor = device.device_descriptor()?;
    for config_id in 0..device_descriptor.num_configurations() {
        let conf_description = device.config_descriptor(config_id)?;
        for device_interface in conf_description.interfaces() {
            for descriptor in device_interface.descriptors() {
                if descriptor.class_code() == 255 {
                    info!("Found interface");

                    for endpoint_descriptor in descriptor.endpoint_descriptors() {
                        info!("Endpoint {:?}", endpoint_descriptor);
                    }
                    let in_endpoint = descriptor
                        .endpoint_descriptors()
                        .find(|endpoint| endpoint.direction() == rusb::Direction::In)
                        .ok_or(DriverError::FailedToOpen)?;
                    info!("In endpoint type is {:?}", in_endpoint.transfer_type());
                    let out_endpoint = descriptor
                        .endpoint_descriptors()
                        .find(|endpoint| endpoint.direction() == rusb::Direction::Out)
                        .ok_or(DriverError::FailedToOpen)?;
                    info!("Out endpoint type is {:?}", out_endpoint.transfer_type());
                    return Ok(Some(Configuration {
                        config_id: conf_description.number(),
                        iface_id: descriptor.interface_number(),
                        in_endpoint_address: in_endpoint.address(),
                        out_endpoint_address: out_endpoint.address(),
                    }));
                }
            }
        }
    }
    Ok(None)
}

fn find_device() -> Result<Option<rusb::Device<rusb::GlobalContext>>> {
    for device in rusb::DeviceList::new()
        .map_err(|_| DriverError::FailedToIterateDevices)?
        .iter()
    {
        let device_description = device
            .device_descriptor()
            .map_err(|_| DriverError::FailedToGetDescription)?;
        info!(
            "vendor ID: {:x} produce ID: {:x}",
            device_description.vendor_id(),
            device_description.product_id()
        );
        if VENDOR_IDS.contains(&device_description.vendor_id()) {
            info!("Found ZSA device");
            if ERGODOX_IDS.contains(&device_description.product_id()) {
                info!("Found ergodox!");
                return Ok(Some(device));
            }
        }
    }
    error!("No ZSA keyboard found");
    Ok(None)
}

use anyhow::Result;
use log::*;
use rusb::{Device, DeviceHandle, DeviceList, Direction, GlobalContext};
use thiserror::Error;

/// legacy - zsa's vendor id
const VENDOR_IDS: [u16; 2] = [0xfeed, 0x3297];
/// legacy - standard - shine - glow
const ERGODOX_IDS: [u16; 4] = [0x1307, 0x4974, 0x4975, 0x4976];
/// legacy - standard - glow
const PLANCK_IDS: [u16; 3] = [0x6060, 0xc6ce, 0xc6cf];
/// mk1
const MOONLANDER_IDS: [u16; 1] = [0x1969];

const EVT_PAIRED: u8 = 0;
const EVT_LAYER: u8 = 2;
const EVT_LIVE_TRAINING: u8 = 3;
const EVT_KEYDOWN: u8 = 17;
const EVT_KEYUP: u8 = 18;
const EVT_LAYOUT_NAME: u8 = 1;
const EVT_LAYOUT_NAME_LEGACY: u8 = 4;

const STATUS_SUCCESS: u8 = 0;

const SEPARATOR: u8 = 254;

#[derive(Debug, PartialEq, Eq)]
pub struct LayoutId {
    pub id: String,
    pub revision: String,
}

impl LayoutId {
    fn decode(text: &str) -> Result<LayoutId> {
        let mut split = text.split('/');
        let id = split.next().ok_or(DriverError::FailedToParseLayout)?;
        let revision = split.next().ok_or(DriverError::FailedToParseLayout)?;
        if split.next().is_none() {
            Ok(LayoutId {
                id: id.to_owned(),
                revision: revision.to_owned(),
            })
        } else {
            Err(DriverError::FailedToParseLayout.into())
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Pair = 0,
    LandingPage = 1,
    #[allow(dead_code)]
    GetLayer = 2,
    LiveTraining = 3,
}

#[derive(Debug, PartialEq, Eq)]
pub struct KeyCode {
    pub column: u8,
    pub row: u8,
}

impl KeyCode {
    pub fn new(column: u8, row: u8) -> Self {
        KeyCode { column, row }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Paired,
    Layer(u8),
    LiveTraining,
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    LayoutName(LayoutId),
}

#[derive(Debug)]
pub struct DeviceConfiguration {
    config_id: u8,
    iface_id: u8,
    in_endpoint_address: u8,
    out_endpoint_address: u8,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DriverError {
    #[error("Failed to iterate devices")]
    FailedToIterateDevices,
    #[error("Failed to get device description")]
    FailedToGetDescription,
    #[error("Device not found")]
    DeviceNotFound,
    #[error("Failed to open device")]
    FailedToOpen,
    #[error("Failed to write")]
    FailedToWrite,
    #[error("Failed decode packet")]
    FailedToDecode,
    #[error("Failed to parse layout description")]
    FailedToParseLayout,
}

pub struct ErgodoxDriver {
    _device: Device<GlobalContext>,
    configuration: DeviceConfiguration,
    handle: DeviceHandle<GlobalContext>,
}

impl ErgodoxDriver {
    pub fn connect_to_first() -> Result<ErgodoxDriver> {
        let first_device = connect_to_first()?.ok_or(DriverError::DeviceNotFound)?;
        let configuration =
            select_interface(&first_device)?.ok_or(DriverError::FailedToGetDescription)?;
        let opened_device = open_device(&first_device, &configuration)?;
        Ok(ErgodoxDriver {
            _device: first_device,
            configuration,
            handle: opened_device,
        })
    }

    pub fn write(&mut self, command: Command) -> Result<()> {
        let written = if let Command::LiveTraining = command {
            self.handle.write_interrupt(
                self.configuration.out_endpoint_address,
                &[command as u8, 2],
                std::time::Duration::from_millis(1000),
            )?
        } else {
            self.handle.write_interrupt(
                self.configuration.out_endpoint_address,
                &[command as u8],
                std::time::Duration::from_millis(1000),
            )?
        };
        if written < 1 {
            return Err(DriverError::FailedToWrite.into());
        }
        trace!("Written bytes {} of {:?}", written, command);
        info!("Written command {:?}", command);
        Ok(())
    }

    pub fn read(&mut self) -> Result<Vec<Event>> {
        let mut buf = [0; 64];
        let read_size = self
            .handle
            .read_interrupt(
                self.configuration.in_endpoint_address,
                &mut buf,
                std::time::Duration::from_millis(1000),
            )
            .unwrap_or_default();
        let data_read = &buf[0..read_size];
        trace!("Read data {:?}", data_read);
        Ok(decode_packet(data_read))
    }
}

fn decode_packet(data: &[u8]) -> Vec<Event> {
    fn unpack_message(payload: &[u8]) -> Result<Event> {
        let status = payload.get(0).ok_or(DriverError::FailedToDecode)?;
        if status != &STATUS_SUCCESS {
            return Err(DriverError::FailedToDecode.into());
        }
        let event = payload.get(1).ok_or(DriverError::FailedToDecode)?;
        let event_message = match *event {
            EVT_LIVE_TRAINING => Event::LiveTraining,
            EVT_PAIRED => Event::Paired,
            EVT_LAYER => {
                let layer = payload.get(2).ok_or(DriverError::FailedToDecode)?;
                Event::Layer(*layer)
            }
            EVT_KEYUP => {
                let col = payload.get(2).ok_or(DriverError::FailedToDecode)?;
                let row = payload.get(3).ok_or(DriverError::FailedToDecode)?;
                Event::KeyUp(KeyCode::new(*col, *row))
            }
            EVT_KEYDOWN => {
                let col = payload.get(2).ok_or(DriverError::FailedToDecode)?;
                let row = payload.get(3).ok_or(DriverError::FailedToDecode)?;
                Event::KeyDown(KeyCode::new(*col, *row))
            }
            EVT_LAYOUT_NAME | EVT_LAYOUT_NAME_LEGACY => {
                let unicode_buffer = payload
                    .get(2..payload.len() - 1)
                    .ok_or(DriverError::FailedToDecode)?;
                let text =
                    std::str::from_utf8(unicode_buffer).map_err(|_| DriverError::FailedToDecode)?;
                Event::LayoutName(LayoutId::decode(&text)?)
            }
            _ => return Err(DriverError::FailedToDecode.into()),
        };
        Ok(event_message)
    }
    data.split(|item| item == &SEPARATOR)
        .map(|message| unpack_message(message))
        .filter_map(Result::ok)
        .collect()
}

fn open_device(
    device: &Device<GlobalContext>,
    config: &DeviceConfiguration,
) -> Result<DeviceHandle<GlobalContext>> {
    trace!("Selected config {:?}", &config);
    let mut device_handle = device.open()?;

    trace!(
        "Has kernel driver {}",
        device_handle.kernel_driver_active(config.iface_id)?
    );

    let active_config = device_handle.active_configuration()?;
    if active_config != config.config_id {
        error!("Desired config not active");
        device_handle
            .set_active_configuration(config.config_id)
            .map_err(|_| DriverError::FailedToOpen)?;
    }
    device_handle.claim_interface(config.iface_id)?;
    Ok(device_handle)
}

fn select_interface(device: &Device<GlobalContext>) -> Result<Option<DeviceConfiguration>> {
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
                        .find(|endpoint| endpoint.direction() == Direction::In)
                        .ok_or(DriverError::FailedToOpen)?;
                    info!("In endpoint type is {:?}", in_endpoint.transfer_type());
                    let out_endpoint = descriptor
                        .endpoint_descriptors()
                        .find(|endpoint| endpoint.direction() == Direction::Out)
                        .ok_or(DriverError::FailedToOpen)?;
                    info!("Out endpoint type is {:?}", out_endpoint.transfer_type());
                    return Ok(Some(DeviceConfiguration {
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

fn connect_to_first() -> Result<Option<Device<GlobalContext>>> {
    for device in DeviceList::new()
        .map_err(|_| DriverError::FailedToIterateDevices)?
        .iter()
    {
        let device_description = device
            .device_descriptor()
            .map_err(|_| DriverError::FailedToGetDescription)?;
        trace!(
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

            if MOONLANDER_IDS.contains(&device_description.product_id()) {
                info!("Found moonlander!");
                return Ok(Some(device));
            }

            if PLANCK_IDS.contains(&device_description.product_id()) {
                info!("Found planck!");
                return Ok(Some(device));
            }
        }
    }
    error!("No ZSA keyboard found");
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_layout_name() {
        let payload = [
            0, 1, 122, 114, 57, 113, 109, 47, 77, 118, 98, 65, 98, 0, 254,
        ];
        let messages = decode_packet(&payload);
        assert!(messages.len() == 1);
        assert_eq!(
            messages[0],
            Event::LayoutName(LayoutId {
                id: "zr9qm".to_owned(),
                revision: "MvbAb".to_owned()
            })
        );
    }

    #[test]
    fn empty_message() {
        let payload = [];
        let messages = decode_packet(&payload);
        assert!(messages.is_empty());
    }

    #[test]
    fn parse_layout() {
        let layout = LayoutId::decode("zr9qm/MvbAb").unwrap();
        assert_eq!(layout.id, "zr9qm");
        assert_eq!(layout.revision, "MvbAb");
    }
}

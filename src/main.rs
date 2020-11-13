mod driver;

use anyhow::Result;
use log::*;
use simplelog::*;

fn main() -> Result<()> {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed)?;
    let mut device = driver::ErgodoxDriver::connect_to_first()?;
    device.write(driver::Command::LandingPage)?;
    device
        .read()?
        .iter()
        .for_each(|message| info!("{:?}", message));
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
    loop {
        device
            .read()?
            .iter()
            .for_each(|message| info!("{:?}", message));
    }
}

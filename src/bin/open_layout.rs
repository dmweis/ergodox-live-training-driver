use anyhow::{anyhow, Result};
use clap::{AppSettings, Clap};
use ergodox_driver::driver;

#[derive(Clap)]
#[clap(version = "0.1", author = "David W <dweis7@gmail.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Args {
    /// Tries to invoke os default browser to open link in
    /// Otherwise prints link to stdout
    #[clap(short = 'b', long)]
    open_browser: bool,
    /// prints messages to stderr
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut device = driver::ErgodoxDriver::connect_to_first()?;
    match device.keyboard_type() {
        driver::KeyboardType::Ergodox => {
            if args.verbose {
                eprintln!("Detected Ergodox");
            }
        }
        _ => {
            if args.verbose {
                eprintln!(
                    "Detected {:?} keyboard, only Ergodox fully supported",
                    device.keyboard_type()
                );
            }
        }
    }
    for _ in 0..10 {
        device.write(driver::Command::LandingPage)?;
        for message in device.read()? {
            if let driver::Event::LayoutName(ref layout_id) = message {
                if args.verbose {
                    eprintln!(
                        "Layout id: {} revision: {}",
                        layout_id.id(),
                        layout_id.revision()
                    );
                }
                // TODO (David): The first path element may have to change for other boards
                // can't test it without them
                let link = format!(
                    "https://configure.zsa.io/ergodox-ez/layouts/{}/{}",
                    layout_id.id(),
                    layout_id.revision()
                );
                if args.open_browser {
                    if webbrowser::open(&link).is_ok() {
                        eprintln!("Opened {}", link);
                    } else {
                        return Err(anyhow!("Failed to open {} in browser", link));
                    }
                } else {
                    println!("{}", link);
                }
                return Ok(());
            }
        }
    }
    Err(anyhow!("Failed to get layout"))
}

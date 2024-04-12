use anyhow::bail;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver},
        peripherals::Peripherals,
        prelude::*,
        spi,
    },
    handle::RawHandle,
    ipv4, ping,
    sys::{self, esp_netif_set_hostname, ESP_OK},
};
use log::info;

mod artnet;
mod webserver;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;
    let sysloop = EspSystemEventLoop::take()?;

    let mut led = {
        let timer = LedcTimerDriver::new(
            peripherals.ledc.timer0,
            &TimerConfig::default().frequency(25.kHz().into()),
        )?;
        LedcDriver::new(peripherals.ledc.channel0, timer, pins.gpio2)?
    };
    led.set_duty(0)?;

    let mut eth = Box::new(esp_idf_svc::eth::EspEth::wrap(
        esp_idf_svc::eth::EthDriver::new_spi(
            spi::SpiDriver::new(
                peripherals.spi2,
                pins.gpio18,
                pins.gpio19,
                Some(pins.gpio23),
                &spi::SpiDriverConfig::new().dma(spi::Dma::Auto(4096)),
            )?,
            pins.gpio26,
            Some(pins.gpio5),
            Some(pins.gpio33),
            esp_idf_svc::eth::SpiEthChipset::W5500,
            20.MHz().into(),
            Some(&[0x02, 0x00, 0x00, 0x12, 0x34, 0x56]),
            None,
            sysloop.clone(),
        )?,
    )?);

    eth_configure(&sysloop, &mut eth)?;

    info!("Done with eth setup");

    webserver::init()?;

    artnet::init(led)?;

    Ok(())
}

fn eth_configure<T>(
    sysloop: &EspSystemEventLoop,
    eth: &mut esp_idf_svc::eth::EspEth<'_, T>,
) -> anyhow::Result<()> {
    info!("Eth created");

    const HOSTNAME: &str = "lunchbox\0";
    unsafe {
        let result = esp_netif_set_hostname(
            eth.netif().handle(),
            core::ffi::CStr::from_bytes_with_nul(HOSTNAME.as_bytes())
                .unwrap()
                .as_ptr(),
        );
        if result != ESP_OK {
            bail!("Failed to set hostname");
        }
    }

    let mut eth = esp_idf_svc::eth::BlockingEth::wrap(eth, sysloop.clone())?;

    info!("Starting eth...");

    eth.start()?;

    info!("Waiting for DHCP lease...");

    eth.wait_netif_up()?;

    let ip_info = eth.eth().netif().get_ip_info()?;

    info!("Eth DHCP info: {:?}", ip_info);

    //ping(ip_info.subnet.gateway)?;

    Ok(())
}

pub fn ping(ip: ipv4::Ipv4Addr) -> anyhow::Result<()> {
    info!("About to do some pings for {:?}", ip);

    let ping_summary = ping::EspPing::default().ping(ip, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        bail!("Pinging IP {} resulted in timeouts", ip);
    }

    info!("Pinging done");

    Ok(())
}

use std::net::UdpSocket;

use artnet_protocol::ArtCommand;
use esp_idf_svc::hal::ledc::LedcDriver;
use log::info;

#[derive(Debug, Clone)]
pub struct DmxState {
    pub data: [u8; 512],
}

static mut DMX_STATE: DmxState = DmxState { data: [0; 512] };

impl DmxState {
    pub fn get() -> &'static DmxState {
        unsafe { &DMX_STATE }
    }

    pub fn set(data: [u8; 512]) {
        unsafe { DMX_STATE.data = data };
    }

    pub fn get_channel(channel: usize) -> u8 {
        unsafe { DMX_STATE.data[channel] }
    }

    pub fn set_channel(channel: usize, value: u8) {
        unsafe { DMX_STATE.data[channel] = value };
    }
}

pub fn init(mut led: LedcDriver) -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:6454")?;
    info!("ArtNet socket bound");

    loop {
        let mut buf = [0; 1024];
        let (length, addr) = socket.recv_from(&mut buf)?;
        let command = ArtCommand::from_buffer(&buf[..length])?;

        info!("Received ArtNet command from {}", addr);
        if let ArtCommand::Output(output) = command {
            let data = output.data.as_ref();
            DmxState::set(data[..512].try_into().unwrap());

            let led_dimmer = data[0] as u32;
            led.set_duty(led_dimmer)?;
        }
    }
}

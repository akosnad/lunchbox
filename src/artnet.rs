use std::net::UdpSocket;

use artnet_protocol::ArtCommand;
use log::info;

use crate::dmx::DmxState;

pub fn init(dmx_state: DmxState) -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:6454").unwrap();
    info!("ArtNet socket bound");

    loop {
        let mut buf = [0; 1024];
        let (length, _addr) = socket.recv_from(&mut buf)?;
        let command = ArtCommand::from_buffer(&buf[..length])?;

        if let ArtCommand::Output(output) = command {
            let data = output.data.as_ref();
            dmx_state.set(data[..512].try_into().unwrap());
        }
    }
}

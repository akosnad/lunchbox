use std::sync::{Arc, Mutex};

use esp_idf_svc::hal::{ledc::LedcDriver, task::thread::ThreadSpawnConfiguration, uart::UartDriver};

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

type SyncedUartDriver<'a> = Arc<Mutex<UartDriver<'a>>>;

pub fn init(led: &'static mut LedcDriver, uart: &'static SyncedUartDriver<'_>) -> anyhow::Result<()> {
    ThreadSpawnConfiguration {
        name: Some("dmx\0".as_bytes()),
        stack_size: 10240,
        ..Default::default()
    }
    .set()?;
    std::thread::spawn(move || loop {
        let uart = uart.clone();
        let loop_result = || -> anyhow::Result<()> {
            let dmx = DmxState::get().data;

            led.set_duty(dmx[0] as u32)?;

            let uart = uart.lock().unwrap();
            uart.write(&DmxState::get().data)?;
            uart.wait_tx_done(150)?;

            uart.clear_rx()?;
            Ok(())
        }();
        if let Err(e) = loop_result {
            log::error!("Error in DMX loop: {:?}", e);
        }
        log::info!("Restarting DMX loop");
    });
    Ok(())
}

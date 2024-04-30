use std::{sync::{Arc, Mutex}, time::Duration};

use esp_idf_svc::hal::{ledc::LedcDriver, task::thread::ThreadSpawnConfiguration, uart::UartDriver};

#[derive(Debug, Clone)]
pub struct DmxState {
    pub data: Arc<Mutex<[u8; 512]>>,
}

impl Default for DmxState {
    fn default() -> Self {
        DmxState {
            data: Arc::new(Mutex::new([0; 512])),
        }
    }
}

impl DmxState {
    pub fn get(&self) -> [u8; 512] {
        let dmx_state = self.data.clone();
        let data = dmx_state.lock().unwrap();
        data.clone()
    }

    pub fn set(&self, data: [u8; 512]) {
        let dmx_state = self.data.clone();
        let mut guard = dmx_state.lock().unwrap();
        *guard = data;
    }

    pub fn get_channel(&self, channel: usize) -> u8 {
        let dmx_state = self.data.clone();
        let data = dmx_state.lock().unwrap();
        data[channel]
    }

    pub fn set_channel(&self, channel: usize, value: u8) {
        let dmx_state = self.data.clone();
        let mut data = dmx_state.lock().unwrap();
        data[channel] = value;
    }
}

const MB_DELAY: u64 = 120;

type SyncedUartDriver<'a> = Arc<Mutex<UartDriver<'a>>>;

pub fn init(led: &'static mut LedcDriver, uart: &'static SyncedUartDriver<'_>, dmx_state: DmxState) -> anyhow::Result<()> {
    ThreadSpawnConfiguration {
        name: Some("dmx\0".as_bytes()),
        stack_size: 10240,
        pin_to_core: Some(esp_idf_svc::hal::cpu::Core::Core1),
        ..Default::default()
    }
    .set()?;

    led.set_duty(255)?;

    std::thread::spawn(move || loop {
        let uart = uart.clone();
        let loop_result = || -> anyhow::Result<()> {
            loop {
                let dmx = dmx_state.get();

                led.set_duty(dmx[0] as u32)?;

                let uart = uart.lock().unwrap();
                uart.write_nb(&dmx)?;

                std::thread::sleep(Duration::from_micros(MB_DELAY));
            }
        }();
        if let Err(e) = loop_result {
            log::error!("Error in DMX loop: {:?}", e);
        }
        log::info!("Restarting DMX loop");
    });
    Ok(())
}

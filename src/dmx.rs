use std::sync::{Arc, Mutex};

use dmx_rdm::{
    consts::{DMX_BAUD, INTER_SLOT_TIME_MILLIS},
    dmx_controller::{DmxController, DmxControllerConfig},
    dmx_uart_driver::{DmxRespUartDriver, DmxUartDriver},
};
use esp_idf_svc::hal::{
    gpio::{Gpio12, Gpio21, Gpio22, Gpio4},
    ledc::LedcDriver,
    task::thread::ThreadSpawnConfiguration,
    uart::{UartConfig, UartDriver, UART2},
};
use esp_idf_svc::sys::EspError;

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

struct LunchboxDmxController {
    uart: UartDriver<'static>,
}
impl LunchboxDmxController {
    fn new(uart: UartDriver<'static>) -> Self {
        Self { uart }
    }

    fn write(&mut self, buffer: &[u8]) -> Result<(), EspError> {
        self.uart.write(buffer)?;
        self.uart.wait_tx_done(INTER_SLOT_TIME_MILLIS as u32)?;
        Ok(())
    }

    fn begin_package(&mut self) -> Result<(), EspError> {
        // send break signal
        self.uart.write(&[0u8; 12])?;
        self.uart.wait_tx_done(10)?;
        Ok(())
    }
}

impl DmxUartDriver for LunchboxDmxController {
    type DriverError = EspError;
}
impl DmxRespUartDriver for LunchboxDmxController {
    fn write_frames(
        &mut self,
        buffer: &[u8],
    ) -> Result<usize, dmx_rdm::dmx_uart_driver::DmxUartDriverError<Self::DriverError>> {
        self.begin_package()?;
        self.write_frames_no_break(buffer)
    }

    fn write_frames_no_break(
        &mut self,
        buffer: &[u8],
    ) -> Result<usize, dmx_rdm::dmx_uart_driver::DmxUartDriverError<Self::DriverError>> {
        if let Err(e) = self.write(buffer) {
            log::error!("Error writing DMX frames: {:?}", e);
            Err(dmx_rdm::dmx_uart_driver::DmxUartDriverError::DriverError(e))
        } else {
            Ok(buffer.len() - 1)
        }
    }
}

pub fn init(
    led: &'static mut LedcDriver,
    uart2: UART2,
    gpio21: Gpio21,
    gpio22: Gpio22,
    gpio12: Gpio12,
    gpio4: Gpio4,
    dmx_state: DmxState,
) -> anyhow::Result<()> {
    let cts: Option<Gpio12> = Some(gpio12);
    let rts: Option<Gpio4> = Some(gpio4);
    let uart = UartDriver::new(
        uart2,
        gpio21,
        gpio22,
        cts,
        rts,
        &UartConfig {
            baudrate: DMX_BAUD.into(),
            data_bits: esp_idf_svc::hal::uart::config::DataBits::DataBits8,
            stop_bits: esp_idf_svc::hal::uart::config::StopBits::STOP2,
            parity: esp_idf_svc::hal::uart::config::Parity::ParityNone,
            flow_control: esp_idf_svc::hal::uart::config::FlowControl::None,
            tx_fifo_size: 0,
            ..Default::default()
        },
    )?;
    let dmx_hw_controller = LunchboxDmxController::new(uart);
    let mut dmx_controller = DmxController::new(dmx_hw_controller, &DmxControllerConfig::default());

    led.set_duty(255)?;

    ThreadSpawnConfiguration {
        name: Some("dmx\0".as_bytes()),
        stack_size: 10240,
        pin_to_core: Some(esp_idf_svc::hal::cpu::Core::Core1),
        ..Default::default()
    }
    .set()?;

    std::thread::spawn(move || loop {
        let loop_result = || -> anyhow::Result<()> {
            loop {
                let dmx = dmx_state.get();

                led.set_duty(dmx[0] as u32)?;

                if let Err(e) = dmx_controller.send_dmx_package(&dmx) {
                    log::error!("Error sending DMX package: {:?}", e);
                }

                //std::thread::sleep(std::time::Duration::from_micros(200));
            }
        }();
        if let Err(e) = loop_result {
            log::error!("Error in DMX loop: {:?}", e);
        }
        log::info!("Restarting DMX loop");
    });
    Ok(())
}

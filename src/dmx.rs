use std::sync::{Arc, Mutex};

use dmx_rdm::{
    consts::{BREAK_MICROS, DMX_BAUD, INTER_SLOT_TIME_MILLIS, MAB_MICROS},
    dmx_controller::{DmxController, DmxControllerConfig},
    dmx_uart_driver::{DmxRespUartDriver, DmxUartDriver},
};
use esp_idf_svc::{hal::{
    delay::Delay, gpio::{Gpio12, Gpio21, Gpio22, Gpio4}, ledc::LedcDriver, task::thread::ThreadSpawnConfiguration, uart::{UartConfig, UartDriver, UART2}
}, sys::{uart_set_line_inverse, uart_signal_inv_t_UART_SIGNAL_INV_DISABLE, uart_signal_inv_t_UART_SIGNAL_TXD_INV}};
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
    delay: Delay,
}
impl LunchboxDmxController {
    fn new(uart: UartDriver<'static>) -> Self {
        Self {
            uart,
            delay: Delay::new(10),
        }
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize, EspError> {
        let mut total_written = 0;
        let total_bytes = buffer.len();
        while total_written < total_bytes {
            let written = self.uart.write_nb(&buffer[total_written..])?;
            total_written += written;
        }
        self.delay.delay_ms(INTER_SLOT_TIME_MILLIS as u32);
        Ok(total_written - 1)
    }

    fn begin_package(&mut self) -> Result<(), EspError> {
        unsafe { uart_set_line_inverse(self.uart.port(), uart_signal_inv_t_UART_SIGNAL_TXD_INV); }
        self.delay.delay_us(BREAK_MICROS as u32);
        unsafe { uart_set_line_inverse(self.uart.port(), uart_signal_inv_t_UART_SIGNAL_INV_DISABLE); }
        self.delay.delay_us(MAB_MICROS as u32);
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
        self.write(buffer).map_err(|e| {
            log::error!("Error writing DMX frames: {:?}", e);
            dmx_rdm::dmx_uart_driver::DmxUartDriverError::TimeoutError
        })
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
            }
        }();
        if let Err(e) = loop_result {
            log::error!("Error in DMX loop: {:?}", e);
        }
        log::info!("Restarting DMX loop");
    });
    Ok(())
}

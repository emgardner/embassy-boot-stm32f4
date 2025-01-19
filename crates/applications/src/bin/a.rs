#![no_std]
#![no_main]

use core::cell::RefCell;
use embassy_boot_stm32::{AlignedBuffer, FirmwareUpdater, FirmwareUpdaterConfig};
use embassy_embedded_hal::adapter::BlockingAsync;
use embassy_embedded_hal::flash::partition::Partition;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::flash::{Flash, WRITE_SIZE};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use {defmt_rtt as _, panic_probe as _};

#[allow(unused)]
static APP_B: &[u8] = include_bytes!("../../b.bin");

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut clock_config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(clock_config);
    let mut button = ExtiInput::new(p.PC13, p.EXTI13, Pull::Up);
    let mut led = Output::new(p.PA5, Level::Low, Speed::Low);
    let flash = Flash::new_blocking(p.FLASH);
    let regions = flash.into_blocking_regions();
    embassy_time::Timer::after_millis(500).await;
    let state_flash: Mutex<
        NoopRawMutex,
        BlockingAsync<embassy_stm32::flash::Bank1Region1<embassy_stm32::flash::Blocking>>,
    > = Mutex::new(BlockingAsync::new(regions.bank1_region1));
    let dfu_flash: Mutex<
        NoopRawMutex,
        BlockingAsync<embassy_stm32::flash::Bank1Region3<embassy_stm32::flash::Blocking>>,
    > = Mutex::new(BlockingAsync::new(regions.bank1_region3));
    let config = FirmwareUpdaterConfig {
        dfu: Partition::new(&dfu_flash, 0x20000, 0x40000),
        state: Partition::new(&state_flash, 0x8000, 0x8000),
    };
    let mut magic = AlignedBuffer([0; WRITE_SIZE]);
    let mut updater = FirmwareUpdater::new(config, &mut magic.0);
    for i in 0..10 {
        led.set_high();
        embassy_time::Timer::after_millis(500).await;
        led.set_low();
        embassy_time::Timer::after_millis(500).await;
    }
    button.wait_for_falling_edge().await;
    let mut offset = 0;
    let mut idx = 0;
    for chunk in APP_B.chunks(2048) {
        let mut buf: [u8; 2048] = [0; 2048];
        buf[..chunk.len()].copy_from_slice(chunk);
        updater.write_firmware(offset, &buf).await.unwrap();
        offset += chunk.len();
    }
    updater.mark_updated().await.unwrap();
    defmt::info!("Update Marked");
    embassy_time::Timer::after_millis(100).await;
    defmt::info!("Resetting");
    embassy_time::Timer::after_millis(100).await;
    cortex_m::peripheral::SCB::sys_reset();
}

#![no_std]
#![no_main]

use core::cell::RefCell;
use cortex_m_rt::{entry, exception};
use embassy_boot_stm32::*;
use embassy_embedded_hal::flash::partition::BlockingPartition;
use embassy_stm32::flash::{Flash, BANK1_REGION3};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
//use panic_probe as _;

#[entry]
fn main() -> ! {
    let clock_config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(clock_config);
    let layout = Flash::new_blocking(p.FLASH).into_blocking_regions();
    let state_flash: Mutex<
        NoopRawMutex,
        RefCell<embassy_stm32::flash::Bank1Region1<embassy_stm32::flash::Blocking>>,
    > = Mutex::new(RefCell::new(layout.bank1_region1));
    let active_flash: Mutex<
        NoopRawMutex,
        RefCell<embassy_stm32::flash::Bank1Region3<embassy_stm32::flash::Blocking>>,
    > = Mutex::new(RefCell::new(layout.bank1_region3));
    let config = BootLoaderConfig {
        active: BlockingPartition::new(&active_flash, 0, 0x20000),
        dfu: BlockingPartition::new(&active_flash, 0x20000, 0x40000),
        state: BlockingPartition::new(&state_flash, 0x8000, 0x8000),
    };
    let active_offset = config.active.offset();
    let bl = BootLoader::prepare::<_, _, _, 2048>(config);
    unsafe { bl.load(BANK1_REGION3.base + active_offset) }
}

#[no_mangle]
#[cfg_attr(target_os = "none", link_section = ".HardFault.user")]
unsafe extern "C" fn HardFault() {
    cortex_m::peripheral::SCB::sys_reset();
}

#[exception]
unsafe fn DefaultHandler(_: i16) -> ! {
    const SCB_ICSR: *const u32 = 0xE000_ED04 as *const u32;
    let irqn = core::ptr::read_volatile(SCB_ICSR) as u8 as i16 - 16;

    panic!("DefaultHandler #{:?}", irqn);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    cortex_m::asm::udf();
}

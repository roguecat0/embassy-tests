#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use static_cell::StaticCell;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");
    let main_tx = mk_static!(Signal<NoopRawMutex, ()>, Signal::new());
    let task_tx = mk_static!(Signal<NoopRawMutex, u64>, Signal::new());

    // TODO: Spawn some tasks
    let _ = spawner;
    spawner.spawn(task1(task_tx, main_tx)).unwrap();

    loop {
        info!("main: waiting for signal");
        let seconds = task_tx.wait().await;
        info!("main: got signal");
        info!("main: waiting for timer");
        Timer::after_secs(seconds).await;
        info!("main: waited {seconds} seconds");
        main_tx.signal(());
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}
#[embassy_executor::task]
async fn task1(
    task_tx: &'static Signal<NoopRawMutex, u64>,
    task_rx: &'static Signal<NoopRawMutex, ()>,
) {
    let mut seconds = 0;
    loop {
        info!("task1: sending signal");
        task_tx.signal(seconds);
        info!("task1: send signal complete");
        info!("task1: waiting for return signal");
        let sig = task_rx.wait().await;
        info!("task1: returned sig = {sig:?}");
        seconds += 1;
    }
}

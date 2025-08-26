#![feature(type_alias_impl_trait)]
#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use core::ptr::addr_of_mut;

use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::{Receiver, Watch};
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, system::CpuControl, system::Stack};

use esp_hal_embassy::Executor;
use log::info;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use esp_backtrace as _;
use static_cell::make_static;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

static mut APP_CORE_STASK: Stack<8192> = Stack::new();

// #[embassy_executor::task(pool_size = 2)]
async fn receive(mut rx: Receiver<'static, CriticalSectionRawMutex, u64, 2>, num_task: usize) {
    loop {
        let num = rx.changed().await;
        if num_task == 2 {
            Timer::after(Duration::from_millis(200)).await;
        }
        info!("receive: {num_task}: got {num}");
    }
}
#[embassy_executor::task]
async fn app_core_task(recvs: [Receiver<'static, CriticalSectionRawMutex, u64, 2>; 2]) {
    match recvs {
        [rx1, rx2] => join(receive(rx1, 1), receive(rx2, 2)).await,
    };
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init([timer0.timer0, timer0.timer1]);
    info!("Embassy initialized!");

    let mut cpu_ctrl = CpuControl::new(peripherals.CPU_CTRL);
    static WATCH: Watch<CriticalSectionRawMutex, u64, 2> = Watch::new();
    let mut rx1 = WATCH.receiver().unwrap();
    let mut rx2 = WATCH.receiver().unwrap();
    let mut tx = WATCH.sender();

    let _guard = cpu_ctrl.start_app_core(unsafe { &mut *(&raw mut APP_CORE_STASK) }, move || {
        info!("in second core");
        let executor: &mut Executor = make_static!(Executor::new());
        executor.run(|spawner| {
            // spawner.spawn(receive(rx1, 1));
            // spawner.spawn(receive(rx2, 2));
            spawner.spawn(app_core_task([rx1, rx2])).unwrap()
        });
    });

    // TODO: Spawn some tasks
    let _ = spawner;

    let mut n = 0;
    let sec = 1;
    loop {
        info!("sending {n}");
        tx.send(n);
        Timer::after(Duration::from_secs(sec)).await;
        info!("after delay: {sec}s");
        n += 1;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}

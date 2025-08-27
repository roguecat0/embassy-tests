#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_sync::pubsub::Publisher;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{AnyPin, Input, InputConfig, OutputPin};
use esp_hal::interrupt::{software::SoftwareInterruptControl, Priority};
use esp_hal::timer::timg::TimerGroup;

use log::info;

use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Subscriber, WaitResult},
};
use embassy_time::{Duration, Instant, Timer, WithTimeout};
use embedded_hal_async::digital::Wait;
use esp_hal_embassy::InterruptExecutor;

use esp_backtrace as _;
use static_cell::StaticCell;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let sw_ints = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init([timer0.timer0, timer0.timer1]);
    static CHANNEL: StaticCell<PubSubChannel<CriticalSectionRawMutex, u32, 5, 1, 1>> =
        StaticCell::new();
    let channel: PubSubChannel<CriticalSectionRawMutex, u32, 5, 1, 1> = PubSubChannel::new();
    let channel = CHANNEL.init(channel);

    let mut rx = channel.subscriber().unwrap();
    let tx = channel.publisher().unwrap();

    // channel

    /// someday maybe figure out how this works. (seems unstable or frigile)
    // let res =
    //     esp_hal::interrupt::enable(esp_hal::peripherals::Interrupt::GPIO, Priority::Priority1);
    // let mut io = Io::new(peripherals.IO_MUX);
    // io.set_interrupt_handler(handler);

    // button
    let mut button = Input::new(
        peripherals.GPIO4,
        InputConfig::default().with_pull(esp_hal::gpio::Pull::Up),
    );
    info!("waiting rising edge");
    button.wait_for_rising_edge().await;

    // TODO: Spawn some tasks
    let _ = spawner;
    spawner.spawn(low_prio(None)).ok();

    static SW_INT_EXT: StaticCell<InterruptExecutor<1>> = StaticCell::new();
    let sw_int_ext = InterruptExecutor::new(sw_ints.software_interrupt1);
    let sw_int_ext = SW_INT_EXT.init(sw_int_ext);

    let pri_spawner = sw_int_ext.start(Priority::Priority1);
    pri_spawner.spawn(higher_prio(Some(button), tx));
    info!("Embassy initialized!");

    info!("Starting low-priority task that isn't actually async");
    loop {
        info!("Blocking: wait");
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {}
        info!("Async: wait");
        let channel_fut = async {
            match rx.next_message().await {
                WaitResult::Lagged(lag) => info!("channel: lagged by {lag}"),
                WaitResult::Message(m) => info!("channel: message {m}"),
            }
        };
        // select(Timer::after(Duration::from_secs(5)), channel_fut).await;
        channel_fut.with_timeout(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn low_prio(mut but: Option<Input<'static>>) {
    let mut ticker = embassy_time::Ticker::every(Duration::from_secs(1));
    loop {
        if let Some(ref mut but) = but {
            info!("prio 0 button");
            but.wait_for_rising_edge().await;
        } else {
            info!("prio 0 task");
            ticker.next().await;
        }
    }
}

#[embassy_executor::task]
async fn higher_prio(
    mut but: Option<Input<'static>>,
    tx: Publisher<'static, CriticalSectionRawMutex, u32, 5, 1, 1>,
) {
    Timer::after(Duration::from_millis(200)).await;
    let mut ticker = embassy_time::Ticker::every(Duration::from_secs(1));
    let mut num: u32 = 0;
    loop {
        num += 1;
        if let Some(ref mut but) = but {
            info!("prio 1 button");
            but.wait_for_rising_edge().await;
        } else {
            info!("prio 1 task");
            ticker.next().await;
        }
        tx.publish_immediate(num)
    }
}

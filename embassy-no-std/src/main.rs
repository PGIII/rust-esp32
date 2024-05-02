#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    embassy::{self},
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
};

#[embassy_executor::task]
async fn run() {
    loop {
        log::info!("Hello from an embassy thread");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[main]
async fn main(spawner: Spawner) {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    esp_println::logger::init_logger_from_env();

    spawner.spawn(run()).ok();

    loop {
        log::info!("Hello from Main");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

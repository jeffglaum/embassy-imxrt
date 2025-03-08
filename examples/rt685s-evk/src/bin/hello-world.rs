#![no_std]
#![no_main]

extern crate embassy_imxrt_examples;

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Timer;
use {panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_imxrt::init(Default::default());

    info!("Hello world");

    loop {
        Timer::after_millis(1000).await;
    }
}

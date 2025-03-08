#![no_std]
#![no_main]

extern crate embassy_imxrt_examples;

//use defmt::info;
use embassy_executor::Spawner;
use embassy_imxrt::gpio;
use embassy_time::Timer;
// N.B. systemview_target cannot be used at the same time as defmt_rtt.
//use rtos_trace;
use systemview_target::SystemView;

static LOGGER: systemview_target::SystemView = systemview_target::SystemView::new();
rtos_trace::global_trace! {SystemView}

struct TraceInfo();

impl rtos_trace::RtosTraceApplicationCallbacks for TraceInfo {
    fn system_description() {}
    fn sysclock() -> u32 {
        64000000
    }
}
rtos_trace::global_application_callbacks! {TraceInfo}

#[inline(never)]
#[no_mangle]
unsafe fn _defmt_release() {}
#[inline(never)]
#[no_mangle]
unsafe fn _defmt_write(bytes: &[u8]) {}
#[inline(never)]
#[no_mangle]
unsafe fn _defmt_acquire() {}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_imxrt::init(Default::default());

    LOGGER.init();

    let mut led = gpio::Output::new(
        p.PIO0_26,
        gpio::Level::Low,
        gpio::DriveMode::PushPull,
        gpio::DriveStrength::Normal,
        gpio::SlewRate::Standard,
    );

    loop {
        rtos_trace::trace::marker(13);
        led.toggle();
        Timer::after_millis(1000).await;
    }
}

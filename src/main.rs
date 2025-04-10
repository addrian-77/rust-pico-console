#![no_std]
#![no_main]
#![allow(unused_imports)]
use embassy_executor::Spawner;
use embassy_rp::spi::{Config, Phase, Polarity, Spi};
use embassy_time::{Instant, Duration, Timer};
use {defmt_rtt as _, panic_probe as _};
use embassy_rp::gpio::{
    Level,
    Output,
};


// Use the logging macros provided by defmt.
use defmt::*;

// Import interrupts definition module
mod irqs;


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    
    let mut led = Output::new(p.PIN_2, Level::Low);

    loop{
        info!("loop\n");
        led.set_high();
        Timer::after(Duration::from_secs(1)).await;
        led.set_low();
        Timer::after(Duration::from_secs(1)).await;
    }
}

use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{
    I2c,
    InterruptHandler as I2cInterruptHandler,
    Config as I2cConfig
};


bind_interrupts!(
    pub(super) struct Irqs {
        I2C0_IRQ => I2cInterruptHandler<embassy_rp::peripherals::I2C0>;
    }
);

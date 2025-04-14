use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::InterruptHandler;

bind_interrupts!(
    pub(super) struct Irqs {
        PIO0_IRQ_0 => InterruptHandler<PIO0>;
    }
);

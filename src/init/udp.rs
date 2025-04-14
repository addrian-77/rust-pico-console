use cyw43::JoinOptions;
use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    StackResources,
};
use embassy_rp::{
    clocks::RoscRng, 
    gpio::Output,
    peripherals::{DMA_CH0, PIO0}, 
};
use {defmt_rtt as _, panic_probe as _};
use defmt::*;
use rand::RngCore;
use static_cell::StaticCell;


type Cyw43Spi = PioSpi<'static, PIO0, 0, embassy_rp::peripherals::DMA_CH0>;

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn wifi_task(runner: cyw43::Runner<'static, Output<'static>, Cyw43Spi>) -> ! {
    runner.run().await
}

pub async fn udp_init(spawner: &Spawner, cyw_pwr: Output<'static>, cyw_spi: PioSpi<'static, PIO0, 0, DMA_CH0>, local_port: u16) -> UdpSocket<'static> {

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000
    let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let (net_device, mut control, runner) = cyw43::new(state, cyw_pwr, cyw_spi, fw).await;

    // run the wifi runtime on an async task
    spawner.spawn(wifi_task(runner)).unwrap();

    // set the country locale matrix and power management
    // wifi_task MUST be running before this gets called
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    info!("wifi module setup complete");

    let mut rng = RoscRng;

    // OPTIONAL: speed up connecting to the network once you know your ip address (via DHCP) by putting your address in LOCAL_IP.txt
    let config = embassy_net::Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner.spawn(net_task(runner)).unwrap();

    // make sure these files exist in your `src` folder
    let wifi_ssid: &str = include_str!("./WIFI_SSID.txt");
    let wifi_password: &str = include_str!("./WIFI_PASSWORD.txt");

    info!("connecting to wifi network '{}'", wifi_ssid);

    loop {
        let options = JoinOptions::new(wifi_password.as_bytes());
        match control.join(wifi_ssid, options).await {
            Ok(_) => {
                info!("connected to wifi network");
                break;
            }
            Err(err) => {
                info!("join failed with status={}, retrying...", err.status);
            }
        }
    }

    info!("waiting for ip config");
    stack.wait_config_up().await;
    info!("config up");

    static RX_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();
    static TX_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();
    static RX_META: StaticCell<[PacketMetadata; 16]> = StaticCell::new();
    static TX_META: StaticCell<[PacketMetadata; 16]> = StaticCell::new();
    let rx_buffer = RX_BUFFER.init([0u8; 4096]);
    let tx_buffer = TX_BUFFER.init([0u8; 4096]);
    let rx_meta = RX_META.init([PacketMetadata::EMPTY; 16]);
    let tx_meta = TX_META.init([PacketMetadata::EMPTY; 16]);

    let mut socket = UdpSocket::new(stack, rx_meta, rx_buffer, tx_meta, tx_buffer);
    socket.bind(local_port).unwrap();

    socket
}

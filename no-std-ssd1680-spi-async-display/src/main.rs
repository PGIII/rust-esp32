#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Delay, Timer};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, Point},
    mono_font::{ascii::FONT_7X14_BOLD, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Triangle},
    text::Text,
    Drawable,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    dma::{Dma, DmaPriority},
    dma_descriptors, embassy,
    gpio::{IO, NO_PIN},
    peripherals::Peripherals,
    prelude::*,
    spi::{
        master::{prelude::*, Spi},
        SpiMode,
    },
    timer::TimerGroup,
    FlashSafeDma,
};
use ssd1680::{
    async_driver::Ssd1680Async,
    graphics::{Display, Display2in13, DisplayRotation},
};

#[main]
async fn main(_spawner: Spawner) {
    let peripherals = Peripherals::take();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let spi = peripherals.SPI2;
    let rst = io.pins.gpio13.into_push_pull_output();
    let dc = io.pins.gpio12.into_push_pull_output();
    let busy = io.pins.gpio14.into_pull_up_input(); //FIXME: what should this be ?
    let sclk = io.pins.gpio10;
    let mosi = io.pins.gpio9;
    let cs = io.pins.gpio11.into_push_pull_output();
    let dma = Dma::new(peripherals.DMA);

    //TODO: why is dma this way?
    #[cfg(any(feature = "esp32", feature = "esp32s2"))]
    let dma_channel = dma.spi2channel;
    #[cfg(not(any(feature = "esp32", feature = "esp32s2")))]
    let dma_channel = dma.channel0;

    //TODO: are these buffers?
    let (mut descriptors, mut rx_descriptors) = dma_descriptors!(3200, 3200);
    let spi = Spi::new(spi, 50_000.kHz(), SpiMode::Mode0, &clocks)
        .with_pins(Some(sclk), Some(mosi), NO_PIN, NO_PIN)
        .with_dma(dma_channel.configure_for_async(
            false,
            &mut descriptors,
            &mut rx_descriptors,
            DmaPriority::Priority0,
        ));
    //This allows accessing data on flash, eg. const and static variables
    //TODO: what size is optimal for this?
    let spi = FlashSafeDma::<_, 3200>::new(spi);
    let spi_device = ExclusiveDevice::new(spi, cs, Delay).unwrap();
    let disp_interface = display_interface_spi::SPIInterface::new(spi_device, dc);
    let mut display_bw = Display2in13::bw();
    let mut delay = Delay;
    let mut ssd1680 = Ssd1680Async::new(disp_interface, busy, rst, &mut delay)
        .await
        .unwrap();
    ssd1680.clear_bw_frame().await.unwrap();
    display_bw.set_rotation(DisplayRotation::Rotate90);
    // background fill
    display_bw
        .fill_solid(&display_bw.bounding_box(), BinaryColor::On)
        .unwrap();

    Text::new(
        "SSD1680 demo",
        Point::new(20, 20),
        MonoTextStyle::new(&FONT_7X14_BOLD, BinaryColor::Off),
    )
    .draw(&mut display_bw)
    .unwrap();
    ssd1680.update_bw_frame(display_bw.buffer()).await.unwrap();
    ssd1680.display_frame(&mut delay).await.unwrap();
    let triangles = [
        Triangle::from_slice(&[Point::new(50, 50), Point::new(0, 100), Point::new(150, 100)]),
        Triangle::from_slice(&[Point::new(75, 75), Point::new(25, 75), Point::new(125, 125)]),
        Triangle::from_slice(&[
            Point::new(100, 100),
            Point::new(0, 50),
            Point::new(150, 150),
        ]),
    ];

    Timer::after_millis(5000).await;
    loop {
        for t in triangles {
            display_bw
                .fill_solid(&display_bw.bounding_box(), BinaryColor::On)
                .unwrap();
            t.into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 1))
                .draw(&mut display_bw)
                .unwrap();
            ssd1680.update_bw_frame(display_bw.buffer()).await.unwrap();
            ssd1680.display_frame(&mut delay).await.unwrap();
            Timer::after_millis(1000).await;
        }
    }
}

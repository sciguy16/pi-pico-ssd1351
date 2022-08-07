// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]
#![no_main]

// Wiring:
// gpio2 -> SCLK
// gpio3 -> MOSI
// gpio4 -> MISO
// gpio5 -> CS
// gpio6 -> DC
// gpio7 -> RST

// For string formatting.
use core::fmt::Write;
use defmt::info;
use defmt_rtt as _;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_9X18_BOLD, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{
        rectangle::Rectangle, PrimitiveStyleBuilder, StrokeAlignment,
    },
    text::{Baseline, Text},
};
use embedded_hal::digital::v2::OutputPin;
use embedded_time::duration::*;
use embedded_time::rate::Extensions;
use panic_halt as _;
use rp_pico::entry;
use rp_pico::hal;
use rp_pico::hal::pac;
use rp_pico::hal::Clock;

/// Entry point to our bare-metal application.
///
/// The `#[entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables are initialised.
///
/// The function configures the RP2040 peripherals,
/// gets a handle on the I2C peripheral,
/// initializes the SSD1306 driver, initializes the text builder
/// and then draws some text on the display.
#[entry]
fn main() -> ! {
    info!("Start boot");

    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    info!("Init SPI");

    // let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS);
    // let mut delay = timer.count_down();
    let mut delay = cortex_m::delay::Delay::new(
        core.SYST,
        clocks.system_clock.freq().integer(),
    );

    let mut led = pins.led.into_push_pull_output();

    // These are implicitly used by the spi driver if they are in the correct mode
    let _spi_sclk = pins.gpio2.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_mosi = pins.gpio3.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_miso = pins.gpio4.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_cs = pins.gpio5.into_push_pull_output();
    let dc = pins.gpio6.into_push_pull_output();
    let mut rst = pins.gpio7.into_push_pull_output();

    // Create an SPI driver instance for the SPI0 device
    let spi = hal::spi::Spi::<_, _, 8>::new(pac.SPI0);

    // Exchange the uninitialised SPI driver for an initialised one
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        20_000_000u32.Hz(),
        // &ssd1351::prelude::SSD1351_SPI_MODE,
        &embedded_hal::spi::MODE_0,
    );

    info!("Init display");

    let mut display: ssd1351::mode::graphics::GraphicsMode<_> =
        ssd1351::builder::Builder::new().connect_spi(spi, dc).into();
    display.init().unwrap();
    info!("Reset display");
    display.reset(&mut rst, &mut delay).unwrap();
    display.init().unwrap();

    // Create a text style for drawing the font:
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_9X18_BOLD)
        .text_color(Rgb565::RED)
        .build();
    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::WHITE)
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();

    // Empty the display:
    DrawTarget::clear(&mut display, Rgb565::BLUE).unwrap();

    // draw border
    display
        .bounding_box()
        .into_styled(border_stroke)
        .draw(&mut display)
        .unwrap();

    // Draw fixed text:
    Text::with_baseline(
        "Hello world!",
        Point::new(10, 0),
        text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();
    Text::with_baseline(
        "Hello Rust!",
        Point::new(10, 20),
        text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    let blue = PrimitiveStyleBuilder::new()
        // .stroke_color(Rgb565::WHITE)
        // .stroke_width(3)
        .fill_color(Rgb565::BLUE)
        .build();

    let mut count: u32 = 0;
    let mut buf = FmtBuf::new();
    loop {
        led.set_high().unwrap();

        // display.reset(&mut rst, &mut delay).unwrap();
        // display.init().unwrap();

        buf.reset();
        // Format some text into a static buffer:
        write!(&mut buf, "counter: {}", count).unwrap();
        info!("Counter: {}", count);
        count += 1;

        // "clear" the bit with the number

        Rectangle::new(Point::new(90, 40), Size::new(30, 18))
            .into_styled(blue)
            .draw(&mut display)
            .unwrap();

        Text::with_baseline(
            buf.as_str(),
            Point::new(10, 40),
            text_style,
            Baseline::Top,
        )
        .draw(&mut display)
        .unwrap();

        // display.flush().unwrap();

        led.set_low().unwrap();
        // Wait a bit:
        // delay.start(500.milliseconds());
        // let _ = nb::block!(delay.wait());
        delay.delay_ms(500);
    }
}

/// This is a very simple buffer to pre format a short line of text
/// limited arbitrarily to 64 bytes.
struct FmtBuf {
    buf: [u8; 64],
    ptr: usize,
}

impl FmtBuf {
    fn new() -> Self {
        Self {
            buf: [0; 64],
            ptr: 0,
        }
    }

    fn reset(&mut self) {
        self.ptr = 0;
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[0..self.ptr]).unwrap()
    }
}

impl core::fmt::Write for FmtBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let rest_len = self.buf.len() - self.ptr;
        let len = if rest_len < s.len() {
            rest_len
        } else {
            s.len()
        };
        self.buf[self.ptr..(self.ptr + len)]
            .copy_from_slice(&s.as_bytes()[0..len]);
        self.ptr += len;
        Ok(())
    }
}

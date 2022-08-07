// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

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
use embassy_executor::executor::Spawner;
use embassy_executor::time::{Delay, Duration, Timer};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::Spi;
use embassy_rp::{spi, Peripherals};
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
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner, p: Peripherals) {
    info!("Start boot");

    info!("Init SPI");

    // let mut led = pins.led.into_push_pull_output();

    let rst = p.PIN_7;
    let display_cs = p.PIN_5;
    let dc = p.PIN_6;
    let miso = p.PIN_4;
    let mosi = p.PIN_3;
    let clk = p.PIN_2;
    let led = p.PIN_25;

    let mut config = spi::Config::default();
    config.frequency = 20_000_000u32;
    let spi = Spi::new(p.SPI0, clk, mosi, miso, config);

    info!("Init display");

    let dc = Output::new(dc, Level::Low);
    let _display_cs = Output::new(display_cs, Level::Low);
    let mut rst = Output::new(rst, Level::Low);
    let mut led = Output::new(led, Level::Low);

    let mut display: ssd1351::mode::graphics::GraphicsMode<_> =
        ssd1351::builder::Builder::new().connect_spi(spi, dc).into();
    display.init().unwrap();
    info!("Reset display");
    display.reset(&mut rst, &mut Delay).unwrap();
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
        led.set_high();

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

        led.set_low();
        // Wait a bit:
        // delay.start(500.milliseconds());
        // let _ = nb::block!(delay.wait());
        // Delay.delay_ms(500);
        Timer::after(Duration::from_secs(1)).await;
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

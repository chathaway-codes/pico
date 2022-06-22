#![no_std]
#![no_main]

#[cfg(not(test))]
use core::panic::PanicInfo;

// peripheral access crate
use rp_pico::{
    self,
    hal::{self, gpio::DynPin, pac, prelude::*},
};

// Time handling traits
use embedded_time::rate::*;

use embedded_hal::{
    // Used to read data from temp sensor
    adc::OneShot,
    // Used to identify input/output pins
    digital::v2::{InputPin, OutputPin},
};

// The macro for our start-up function
use cortex_m_rt::entry;

use cortex_m::delay::Delay;

use rand::prelude::*;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Watchdog is a HW timer that counts down, and if it
    // reaches 0, restarts the program. We use this as a timer
    // when creating our clock.
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

    // create the delay
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    // get access to the GPIO pins
    let sio = hal::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut rng = create_rng(pac.ADC, &mut pac.RESETS, pac.ROSC);

    // NOTE: we don't have an allocator setup, so we can't use Vec. Here and below, we
    // use fixed-length arrays.
    let mut led_pins = [
        DynPin::from(pins.gpio2.into_push_pull_output()),
        DynPin::from(pins.gpio0.into_push_pull_output()),
        DynPin::from(pins.gpio1.into_push_pull_output()),
        DynPin::from(pins.gpio3.into_push_pull_output()),
    ];
    let input_pins = [
        DynPin::from(pins.gpio10.into_pull_down_input()),
        DynPin::from(pins.gpio11.into_pull_down_input()),
        DynPin::from(pins.gpio12.into_pull_down_input()),
        DynPin::from(pins.gpio13.into_pull_down_input()),
    ];

    // Draw the startup routine
    draw_startup(&mut led_pins, &mut delay, 100);

    let mut i = 0;
    let mut sequence = [0; 10];
    loop {
        let new_val = (rng.next_u32() % led_pins.len() as u32) as usize;
        sequence[i] = new_val;
        // show the player the sequence
        draw_sequence(&sequence[0..i + 1], &mut led_pins, &mut delay, 500);
        // collect input from user
        for j in 0..i + 1 {
            // Prompts for input, and echos it back to the player
            let val = wait_for_input(&input_pins, &mut led_pins);
            // Give the player a breath to let go of the button
            delay.delay_ms(50);
            if val != sequence[j] {
                // player lost; show them how far they got
                led_pins.iter_mut().for_each(|x| x.set_low().unwrap());
                let percent = ((i as f32 / sequence.len() as f32) * led_pins.len() as f32) as usize;
                for led in led_pins.iter_mut().take(percent) {
                    led.set_high().unwrap();
                }
                loop {
                    delay.delay_ms(1000);
                }
            }
        }
        i += 1;
        if i == sequence.len() {
            // game over! player won, show them something cool
            loop {
                draw_startup(&mut led_pins, &mut delay, 100);
            }
        }
    }
}

fn draw_sequence(sequence: &[usize], leds: &mut [DynPin], delay: &mut Delay, speed: u32) {
    leds.iter_mut().for_each(|x| x.set_low().unwrap());
    delay.delay_ms(speed);
    for pos in sequence {
        leds[*pos].set_high().unwrap();
        delay.delay_ms(speed);
        leds[*pos].set_low().unwrap();
        delay.delay_ms(speed);
    }
}

fn wait_for_input(inputs: &[DynPin], leds: &mut [DynPin]) -> usize {
    let mut input: Option<usize> = None;
    leds.iter_mut().for_each(|x| x.set_low().unwrap());
    while input.is_none() {
        for (i, pin) in inputs.iter().enumerate() {
            while pin.is_high().unwrap() {
                input = Some(i);
                leds[i].set_high().unwrap();
            }
            if input.is_some() {
                break;
            }
        }
    }
    let i = input.unwrap();
    leds[i].set_low().unwrap();
    i
}

fn draw_startup(leds: &mut [DynPin], delay: &mut Delay, speed: u32) {
    leds.iter_mut().for_each(|x| x.set_low().unwrap());
    let mut last = 0;
    for i in 0..leds.len() {
        leds[last].set_low().unwrap();
        leds[i].set_high().unwrap();
        last = i;
        delay.delay_ms(speed);
    }
    for i in (0..leds.len() - 1).rev() {
        leds[last].set_low().unwrap();
        leds[i].set_high().unwrap();
        last = i;
        delay.delay_ms(speed);
    }
    // flash all lights twice
    leds.iter_mut().for_each(|x| x.set_low().unwrap());
    delay.delay_ms(speed);
    leds.iter_mut().for_each(|x| x.set_high().unwrap());
    delay.delay_ms(speed);
    leds.iter_mut().for_each(|x| x.set_low().unwrap());
    delay.delay_ms(speed);
    leds.iter_mut().for_each(|x| x.set_high().unwrap());
    delay.delay_ms(speed);
    leds.iter_mut().for_each(|x| x.set_low().unwrap());
}

fn create_rng(adc: pac::ADC, resets: &mut pac::RESETS, rosc: pac::ROSC) -> rand::rngs::SmallRng {
    // Randomness is hard to come by on this chip; use temperature + OSC bits
    let mut adc = hal::adc::Adc::new(adc, resets);
    let mut temp_sensor = adc.enable_temp_sensor();
    let temp: u16 = adc.read(&mut temp_sensor).unwrap();
    adc.disable_temp_sensor(temp_sensor);

    let osc = hal::rosc::RingOscillator::new(rosc).initialize();
    let rng = rand::rngs::SmallRng::from_seed([
        temp as u8,
        (temp >> 8) as u8,
        // We can't really get more randomness from the 16 bits we had, so just pass in osc.
        // I wonder if the OSC changes fast enough that this won't all be the same value?
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
        osc.get_random_bit() as u8,
    ]);
    osc.disable();
    rng
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

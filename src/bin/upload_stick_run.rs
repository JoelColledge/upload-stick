extern crate upload_stick_lib;

use std::fs::File;
use std::io::prelude::*;
use std::option::Option;
use std::path::Path;
use std::process::Command;
use upload_stick_lib::command_stdout;

const GPIO_GREEN: &'static str = "23";
const GPIO_YELLOW: &'static str = "25";
const GPIO_BLUE: &'static str = "12";
const GPIO_RED: &'static str = "20";
const GPIO_ALL: [&'static str; 4] = [GPIO_GREEN, GPIO_YELLOW, GPIO_BLUE, GPIO_RED];

fn main() {
    println!("Starting monitoring and upload of files");

    prepare_leds().unwrap();
    set_leds(&[GPIO_GREEN, GPIO_BLUE]).unwrap();

    // loop {
    //     set_leds(&[]).unwrap();
    //     wait_for_idle();
    //     set_leds(&[]).unwrap();
    //     upload_new_files();
    // }
}

fn sys_gpio() -> Box<Path> {
    return Box::from(Path::new("/sys/class/gpio"));
}

fn sys_gpio_export() -> Box<Path> {
    return sys_gpio().join("export").into_boxed_path();
}

fn sys_gpio_pin(gpio: &str) -> Box<Path> {
    return sys_gpio().join(String::from("gpio") + gpio).into_boxed_path();
}

fn sys_gpio_direction(gpio: &str) -> Box<Path> {
    return sys_gpio_pin(gpio).join("direction").into_boxed_path();
}

fn sys_gpio_value(gpio: &str) -> Box<Path> {
    return sys_gpio_pin(gpio).join("value").into_boxed_path();
}

fn prepare_leds() -> std::io::Result<()> {
    for gpio in GPIO_ALL.iter() {
        if sys_gpio_pin(gpio).exists() {
            println!("GPIO {} already exported", gpio);
        } else {
            println!("Exporting GPIO {}", gpio);
            File::create(sys_gpio_export())?.write_all(gpio.as_bytes())?;
        }

        File::create(sys_gpio_direction(gpio))?.write_all(b"out")?;
    }
    Ok(())
}

fn set_leds(gpios: &[&str]) -> std::io::Result<()> {
    for gpio in GPIO_ALL.iter() {
        let value = if gpios.contains(gpio) { b"1" } else { b"0" };
        File::create(sys_gpio_value(gpio))?.write_all(value)?;
    }
    Ok(())
}

// fn wait_for_idle() {
//     //
// }
//
// fn upload_new_files() {
//     // TODO: Snapshot, mount, check for new files, upload
// }

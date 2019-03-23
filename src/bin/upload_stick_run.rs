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

    // loop {
        set_leds(&[GPIO_GREEN]).unwrap();
        wait_for_idle().unwrap();
        set_leds(&[GPIO_YELLOW]).unwrap();
        // upload_new_files();
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

fn sys_block_stat() -> Box<Path> {
    return Box::from(Path::new("/sys/block/dm-0/stat"));
}

fn stat_find_writes(stat_output: &str) -> Result<u64, String> {
    return stat_output
        .split_whitespace()
        .nth(6).ok_or("No element 6 in stat output".to_string())
        .and_then(|writes| writes.parse::<u64>().map_err(|err| err.to_string()))
}

fn wait_for_idle() -> std::io::Result<()> {
    let mut stat_file = File::open(sys_block_stat())?;
    loop {
        let mut stat_output = String::new();
        stat_file.seek(std::io::SeekFrom::Start(0))?;
        stat_file.read_to_string(&mut stat_output)?;
        let writes = stat_find_writes(&stat_output)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        println!("Writes {}", writes);
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

// fn upload_new_files() {
//     // TODO: Snapshot, mount, check for new files, upload
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_find_writes() {
        let writes = stat_find_writes("     158        0    20232      800     2567        0    20536  1279180        0     1650  1279980");
        assert_eq!(writes, Ok(20536));
    }
}

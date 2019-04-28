extern crate upload_stick;

use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use upload_stick::upload_command::*;
use upload_stick::upload_db;

const GPIO_GREEN: &'static str = "23";
const GPIO_YELLOW: &'static str = "25";
const GPIO_BLUE: &'static str = "12";
const GPIO_RED: &'static str = "20";
const GPIO_ALL: [&'static str; 4] = [GPIO_GREEN, GPIO_YELLOW, GPIO_BLUE, GPIO_RED];

fn main() {
    println!("Starting monitoring and upload of files");

    process::exit(match run() {
        Ok(_) => {
            // non-zero because this should never terminate
            2
        },
        Err(err) => {
            println!("Failed to run: {}", err);
            1
        }
    });
}

fn run() -> Result<()> {
    prepare_leds()?;
    set_leds(&[GPIO_GREEN])?;

    clean_snapshot()?;

    match main_loop() {
        Ok(_) => {
            println!("File monitoring finished unexpectedly");
        },
        Err(err) => {
            println!("Monitoring and upload failed: {}", err);
        }
    }

    set_leds(&[GPIO_RED])?;
    Ok(())
}

fn clean_snapshot() -> Result<()> {
    command_ignore_output(
        Command::new("umount").arg("/mnt")
    )?;

    unmap_partition("mass_storage_snap_partition", CommandCheck::IgnoreOutput)?;

    command_ignore_output(
        Command::new("lvremove")
            .arg("--yes")
            .arg("data/mass_storage_snap")
    )?;

    Ok(())
}

fn main_loop() -> Result<()> {
    loop {
        println!("upload_new_files");
        upload_new_files()?;
        println!("wait_for_active");
        wait_for_active()?;
        println!("wait_for_idle");
        wait_for_idle()?;
    }
}

fn sys_gpio() -> PathBuf {
    PathBuf::from("/sys/class/gpio")
}

fn sys_gpio_export() -> PathBuf {
    sys_gpio().join("export")
}

fn sys_gpio_pin(gpio: &str) -> PathBuf {
    sys_gpio().join(String::from("gpio") + gpio)
}

fn sys_gpio_direction(gpio: &str) -> PathBuf {
    sys_gpio_pin(gpio).join("direction")
}

fn sys_gpio_value(gpio: &str) -> PathBuf {
    sys_gpio_pin(gpio).join("value")
}

fn prepare_leds() -> Result<()> {
    prepare_leds_io().map_err(|io_error| Error::LedSysfs(io_error))
}

fn prepare_leds_io() -> std::io::Result<()> {
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

fn set_leds(gpios: &[&str]) -> Result<()> {
    set_leds_io(gpios).map_err(|io_error| Error::LedSysfs(io_error))
}

fn set_leds_io(gpios: &[&str]) -> std::io::Result<()> {
    for gpio in GPIO_ALL.iter() {
        let value = if gpios.contains(gpio) { b"1" } else { b"0" };
        File::create(sys_gpio_value(gpio))?.write_all(value)?;
    }
    Ok(())
}

fn sys_block_stat(minor: u64) -> PathBuf {
    PathBuf::from(format!("/sys/block/dm-{}/stat", minor))
}

fn stat_find_writes(stat_output: &str) -> Result<u64> {
    stat_output
        .split_whitespace()
        .nth(6).ok_or(Error::StatWritesNotFound(stat_output.to_string()))
        .and_then(|writes| writes.parse::<u64>().map_err(|err| Error::StatWritesParse(err)))
}

fn find_mass_storage_minor() -> Result<u64> {
    let lvs_output = command_stdout(
        Command::new("lvs")
            .arg("-o").arg("kernel_minor")
            .arg("--noheadings")
            .arg("data/mass_storage_root")
    )?;

    lvs_output.trim().parse::<u64>()
        .map_err(|err| Error::LvsMinorParse(err))
}

fn wait_for_write_condition<F>(seconds: usize, mut f: F) -> Result<()>
    where F: FnMut(&u64, &u64) -> bool
{
    let minor = find_mass_storage_minor()?;
    let mut stat_file = File::open(sys_block_stat(minor))
        .map_err(|io_error| Error::StatWritesSysfs(io_error))?;
    let mut history = std::collections::VecDeque::new();
    let history_size = seconds + 1;
    loop {
        let mut stat_output = String::new();
        stat_file.seek(std::io::SeekFrom::Start(0))
            .map_err(|io_error| Error::StatWritesSysfs(io_error))?;
        stat_file.read_to_string(&mut stat_output)
            .map_err(|io_error| Error::StatWritesSysfs(io_error))?;
        let writes = stat_find_writes(&stat_output)?;
        println!("Writes {}", writes);
        history.push_front(writes);
        history.truncate(history_size);

        if history.len() == history_size && f(history.back().unwrap(), history.front().unwrap()) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

fn wait_for_idle() -> Result<()> {
    wait_for_write_condition(6, |old_writes, new_writes| old_writes == new_writes)
}

fn wait_for_active() -> Result<()> {
    wait_for_write_condition(1, |old_writes, new_writes| old_writes != new_writes)
}

fn is_wav(file_path: &Path) -> bool {
    return match file_path.extension() {
        None => false,
        Some(extension) => extension == "wav"
    };
}

fn upload_new_files() -> Result<()> {
    command_stdout(
        Command::new("lvcreate")
            .arg("--snapshot")
            .arg("--extents").arg("100%FREE")
            .arg("--name").arg("mass_storage_snap")
            .arg("data/mass_storage_root")
    )?;

    map_lv_partition("mass_storage_snap", "mass_storage_snap_partition", MapMode::ReadOnly)?;

    command_stdout(
        Command::new("mount")
            .arg("/dev/mapper/mass_storage_snap_partition")
            .arg("/mnt")
            .arg("-o").arg("ro")
    )?;

    for dir_entry in std::fs::read_dir(Path::new("/mnt"))
            .map_err(|io_error| Error::IteratingDirectory(io_error))? {
        let dir_entry = dir_entry
            .map_err(|io_error| Error::IteratingDirectory(io_error))?;

        if is_wav(&dir_entry.path()) {
            let upload_entry = upload_db::from_dir_entry(&dir_entry).unwrap();

            if !upload_db::is_uploaded(&upload_entry).unwrap() {
                println!("new file: {:?}", dir_entry.path());
                let tmp_path = Path::new("/tmp/upload-stick");

                if tmp_path.exists() {
                    fs::remove_dir_all(tmp_path).unwrap();
                }
                fs::create_dir(tmp_path).unwrap();

                let mut output_path = tmp_path.join(dir_entry.path().file_stem().expect("No file name")).with_extension("ogg");
                println!("encode {:?} to {:?}", dir_entry.path(), output_path);
                set_leds(&[GPIO_YELLOW])?;
                command_stdout(
                    Command::new("oggenc")
                        .arg("--quality").arg("6")
                        .arg("--downmix")
                        .arg("--output").arg(&output_path)
                        .arg(dir_entry.path())
                )?;

                println!("upload {:?}", output_path);
                set_leds(&[GPIO_BLUE])?;
                command_stdout(
                    Command::new("rclone")
                        .arg("copy")
                        .arg(&output_path)
                        .arg("upload:/Auto_Upload/")
                )?;

                upload_db::set_uploaded(&upload_entry).unwrap();
            }
        }
    }

    set_leds(&[GPIO_GREEN])?;

    command_stdout(
        Command::new("umount").arg("/mnt")
    )?;

    unmap_partition("mass_storage_snap_partition", CommandCheck::ExpectZeroExitCode)?;

    command_stdout(
        Command::new("lvremove")
            .arg("--yes")
            .arg("data/mass_storage_snap")
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_find_writes() {
        let writes = stat_find_writes("     158        0    20232      800     2567        0    20536  1279180        0     1650  1279980").unwrap();
        assert_eq!(writes, Ok(20536));
    }
}

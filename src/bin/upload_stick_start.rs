extern crate upload_stick;

use std::process::Command;
use upload_stick::upload_command::{Result, command_stdout};

fn main() {
    println!("Cleaning and starting mass storage volume");

    match start() {
        Ok(_) => {
            println!("Successfully started mass storage volume");
        },
        Err(err) => {
            println!("Failed to start mass storage volume: {}", err);
        }
    }
}

fn start() -> Result<()> {
    // TODO: Clean old files to free up space

    println!("Enabling mass storage module");
    command_stdout(
        Command::new("modprobe")
            .arg("g_mass_storage")
            .arg("file=/dev/data/mass_storage_root")
            .arg("stall=0")
            .arg("removable=yes")
    )?;

    Ok(())
}

extern crate upload_stick_lib;

use std::process::Command;
use upload_stick_lib::command_stdout;

fn main() {
    println!("Cleaning and starting mass storage volume");

    // TODO: Clean old files to free up space

    println!("Enabling mass storage module");
    command_stdout(
        Command::new("modprobe")
            .arg("g_mass_storage")
            .arg("file=/dev/data/mass_storage_root")
            .arg("stall=0")
            .arg("removable=yes")
    );
}

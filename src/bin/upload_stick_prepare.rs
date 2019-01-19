extern crate upload_stick_lib;

use std::process::Command;
use std::str;
use upload_stick_lib::command_stdout;

fn main() {
    println!("Setting up mass storage volume");

    println!("Getting SD partitions");
    let parted_output = command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("--machine")
            .arg("/dev/mmcblk0")
            .arg("unit").arg("MB")
            .arg("print").arg("free")
    );

    let (from_str, to_str) = parted_find_last_free(&parted_output);

    println!("Adding SD partition");
    command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("/dev/mmcblk0")
            .arg("mkpart").arg("primary").arg("").arg(from_str).arg(to_str)
    );

    println!("Making PV");
    command_stdout(Command::new("pvcreate").arg("/dev/mmcblk0p3"));

    println!("Making VG");
    command_stdout(Command::new("vgcreate").arg("data").arg("/dev/mmcblk0p3"));

    println!("Making LV");
    command_stdout(
        Command::new("lvcreate")
            .arg("--extents").arg("50%FREE").arg("--name").arg("mass_storage_root").arg("data")
    );

    println!("Writing mass storage partition label");
    command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("/dev/data/mass_storage_root")
            .arg("mklabel").arg("msdos")
    );

    println!("Adding mass storage partition");
    command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("/dev/data/mass_storage_root")
            .arg("--")
            .arg("mkpart").arg("primary").arg("fat32").arg("4MiB").arg("-1s")
    );

    println!("Getting storage partition");
    let storage_parted_output = command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("--machine")
            .arg("/dev/data/mass_storage_root")
            .arg("unit").arg("s")
            .arg("print")
    );

    let (storage_from, storage_length) = parted_find_first_start_length(&storage_parted_output);

    println!("Creating mapping to storage partition");
    command_stdout(
        Command::new("dmsetup")
            .arg("create")
            .arg("--table")
            .arg(format!(
                "0 {} linear /dev/data/mass_storage_root {}",
                drop_units(&storage_length),
                drop_units(&storage_from)
            ))
            .arg("mass_storage_partition")
    );

    println!("Initializing file system");
    command_stdout(
        Command::new("mkfs.fat")
            .arg("/dev/mapper/mass_storage_partition")
            .arg("-F").arg("32")
            .arg("-n").arg("PI_UPLOAD")
    );

    command_stdout(&mut Command::new("sync"));

    println!("Removing mapping to storage partition");
    command_stdout(
        Command::new("dmsetup")
            .arg("remove")
            .arg("mass_storage_partition")
    );
}

fn parted_find_last_free(parted_output: &str) -> (String, String) {
    let free_line = parted_output.lines()
        .filter(|line| line.trim().ends_with("free;"))
        .last()
        .expect(&format!("no 'free' lines in parted output: {}", parted_output));

    match free_line.split(":").take(3).collect::<Vec<&str>>().as_slice() {
        [_, from, to] => (String::from(from.trim()), String::from(to.trim())),
        _ => panic!("'free' line in parted output does not contain expected fields: {}", free_line)
    }
}

fn parted_find_first_start_length(parted_output: &str) -> (String, String) {
    let part_line = parted_output.lines()
        .filter(|line| line.trim().starts_with("1:"))
        .next()
        .expect(&format!("no '1:' lines in parted output: {}", parted_output));

    match part_line.split(":").take(4).collect::<Vec<&str>>().as_slice() {
        [_, from, _, length] => (String::from(from.trim()), String::from(length.trim())),
        _ => panic!("'1:' line in parted output does not contain expected fields: {}", part_line)
    }
}

fn drop_units(string: &str) -> String {
    string.chars().take_while(|&c| char::is_numeric(c)).collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parted_find_last_free() {
        let (from, to) = parted_find_last_free("
            BYT;
            /dev/mmcblk0:31915MB:sd/mmc:512:512:msdos:SD ACLCD:;
            1:0.02MB:4.19MB:4.18MB:free;
            1:4.19MB:46.1MB:41.9MB:fat16::boot, lba;
            2:46.1MB:201MB:155MB:ext3::;
            1:201MB:31915MB:31714MB:free;
        ");
        assert_eq!(from, "201MB");
        assert_eq!(to, "31915MB");
    }

    #[test]
    fn test_parted_find_first_start_length() {
        let (from, length) = parted_find_first_start_length("
            BYT;
            /dev/dm-4:30900224s:unknown:512:512:msdos:Unknown:;
            1:8192s:30900223s:30892032s:::lba;
        ");
        assert_eq!(from, "8192s");
        assert_eq!(length, "30892032s");
    }

    #[test]
    fn test_drop_units() {
        assert_eq!(drop_units("30892032s"), "30892032");
    }
}

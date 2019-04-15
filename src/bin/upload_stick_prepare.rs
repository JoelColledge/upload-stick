extern crate upload_stick;

use std::process::Command;
use std::str;
use upload_stick::upload_command::*;

fn main() {
    println!("Preparing mass storage volume");

    match prepare() {
        Ok(_) => {
            println!("Successfully prepared mass storage volume");
        },
        Err(err) => {
            println!("Failed to prepare mass storage volume: {}", err);
        }
    }
}

fn prepare() -> Result<()> {
    println!("Resizing root partition");
    command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("/dev/mmcblk0")
            .arg("resizepart").arg("2").arg("2GiB")
    )?;

    println!("Resizing root file system");
    command_stdout(Command::new("resize2fs").arg("/dev/mmcblk0p2"))?;

    println!("Getting SD partitions");
    let parted_output = command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("--machine")
            .arg("/dev/mmcblk0")
            .arg("unit").arg("MB")
            .arg("print").arg("free")
    )?;

    let (from_str, to_str) = parted_find_last_free(&parted_output)?;

    println!("Adding SD partition");
    command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("/dev/mmcblk0")
            .arg("mkpart").arg("primary").arg("").arg(from_str).arg(to_str)
    )?;

    println!("Making PV");
    command_stdout(Command::new("pvcreate").arg("/dev/mmcblk0p3"))?;

    println!("Making VG");
    command_stdout(Command::new("vgcreate").arg("data").arg("/dev/mmcblk0p3"))?;

    println!("Making LV");
    command_stdout(
        Command::new("lvcreate")
            .arg("--extents").arg("70%FREE").arg("--name").arg("mass_storage_root").arg("data")
    )?;

    println!("Writing mass storage partition label");
    command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("/dev/data/mass_storage_root")
            .arg("mklabel").arg("msdos")
    )?;

    println!("Adding mass storage partition");
    command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("/dev/data/mass_storage_root")
            .arg("--")
            .arg("mkpart").arg("primary").arg("fat32").arg("4MiB").arg("-1s")
    )?;

    map_lv_partition("mass_storage_root", "mass_storage_partition")?;

    println!("Initializing file system");
    command_stdout(
        Command::new("mkfs.fat")
            .arg("/dev/mapper/mass_storage_partition")
            .arg("-F").arg("32")
            .arg("-n").arg("PI_UPLOAD")
    )?;

    command_stdout(&mut Command::new("sync"))?;

    unmap_partition("mass_storage_partition")?;

    Ok(())
}

fn parted_find_last_free(parted_output: &str) -> Result<(String, String)> {
    let free_line = parted_output.lines()
        .filter(|line| line.trim().ends_with("free;"))
        .last()
        .ok_or(Error::PartitionFreeNotFound(parted_output.to_string()))?;

    match free_line.split(":").take(3).collect::<Vec<&str>>().as_slice() {
        [_, from, to] => Ok((String::from(from.trim()), String::from(to.trim()))),
        _ => Err(Error::PartitionFreeFieldsNotFound(free_line.to_string()))
    }
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
        ").unwrap();
        assert_eq!(from, "201MB");
        assert_eq!(to, "31915MB");
    }
}

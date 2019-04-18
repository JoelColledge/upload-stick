use std::fmt;
use std::io;
use std::num;
use std::string;
use std::result;
use std::process::Command;

pub enum Error {
    CommandNonZeroExitCode(i32),
    CommandTerminatedBySignal,
    CommandOther(io::Error),
    StdoutNotUtf8(string::FromUtf8Error),
    Partition1NotFound(String),
    PartitionFieldsNotFound(String),
    PartitionFreeNotFound(String),
    PartitionFreeFieldsNotFound(String),
    LedSysfs(io::Error),
    StatWritesNotFound(String),
    StatWritesParse(num::ParseIntError),
    StatWritesSysfs(io::Error),
    IteratingDirectory(io::Error),
    LvsMinorParse(num::ParseIntError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CommandNonZeroExitCode(code) => write!(f, "Command terminated with exit code {}", code),
            Error::CommandTerminatedBySignal => write!(f, "Command terminated by signal"),
            Error::CommandOther(err) => write!(f, "I/O error executing command: {}", err),
            Error::StdoutNotUtf8(err) => write!(f, "Could not parse stdout as UTF-8: {}", err),
            Error::Partition1NotFound(output) => write!(f, "Could not find partition 1 in output: {}", output),
            Error::PartitionFieldsNotFound(line) => write!(f, "Could not find required partition fields: {}", line),
            Error::PartitionFreeNotFound(output) => write!(f, "Could not find space for partition in output: {}", output),
            Error::PartitionFreeFieldsNotFound(line) => write!(f, "Could not find required free space fields: {}", line),
            Error::LedSysfs(err) => write!(f, "I/O error controlling LEDs over sysfs: {}", err),
            Error::StatWritesNotFound(line) => write!(f, "Could not find writes field in stat output: {}", line),
            Error::StatWritesParse(err) => write!(f, "Could not parse stat writes field: {}", err),
            Error::StatWritesSysfs(err) => write!(f, "I/O error watching stat writes over sysfs: {}", err),
            Error::IteratingDirectory(err) => write!(f, "I/O error iterating over directory: {}", err),
            Error::LvsMinorParse(err) => write!(f, "Could not parse device minor number from lvs: {}", err),
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

pub fn command_stdout(command: &mut Command) -> Result<String> {
    let output = command
        .output()
        .map_err(|io_error| Error::CommandOther(io_error))?;

    let stdout = String::from_utf8(output.stdout)
        .map_err(|utf8_error| Error::StdoutNotUtf8(utf8_error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Failed external process; stdout:\n{}", stdout);
        eprintln!("Failed external process; stderr:\n{}", stderr);
        return Err(match output.status.code() {
            Some(code) => Error::CommandNonZeroExitCode(code),
            None => Error::CommandTerminatedBySignal
        });
    };

    Ok(stdout)
}

pub fn map_lv_partition(lv_name: &str, mapped_name: &str) -> Result<()> {
    println!("Getting storage partition");
    let storage_parted_output = command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("--machine")
            .arg(format!("/dev/data/{}", lv_name))
            .arg("unit").arg("s")
            .arg("print")
    )?;

    let (storage_from, storage_length) = parted_find_first_start_length(&storage_parted_output)?;

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
            .arg(mapped_name)
    )?;

    Ok(())
}

pub fn unmap_partition(mapped_name: &str) -> Result<()> {
    println!("Removing mapping to storage partition");
    command_stdout(
        Command::new("dmsetup")
            .arg("remove")
            .arg(mapped_name)
    )?;

    Ok(())
}

fn parted_find_first_start_length(parted_output: &str) -> Result<(String, String)> {
    let part_line = parted_output.lines()
        .filter(|line| line.trim().starts_with("1:"))
        .next()
        .ok_or(Error::Partition1NotFound(parted_output.to_string()))?;

    match part_line.split(":").take(4).collect::<Vec<&str>>().as_slice() {
        [_, from, _, length] => Ok((String::from(from.trim()), String::from(length.trim()))),
        _ => Err(Error::PartitionFieldsNotFound(part_line.to_string()))
    }
}

fn drop_units(string: &str) -> String {
    string.chars().take_while(|&c| char::is_numeric(c)).collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parted_find_first_start_length() {
        let (from, length) = parted_find_first_start_length("
            BYT;
            /dev/dm-4:30900224s:unknown:512:512:msdos:Unknown:;
            1:8192s:30900223s:30892032s:::lba;
        ").unwrap();
        assert_eq!(from, "8192s");
        assert_eq!(length, "30892032s");
    }

    #[test]
    fn test_drop_units() {
        assert_eq!(drop_units("30892032s"), "30892032");
    }
}

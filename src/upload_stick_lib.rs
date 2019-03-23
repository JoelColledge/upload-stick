use std::process::Command;

pub fn command_stdout(command: &mut Command) -> String {
    let output = command
        .output()
        .expect("Failed to execute process");

    if !output.status.success() {
        match output.status.code() {
            Some(code) => panic!("External process exited with status code: {}", code),
            None => panic!("External process terminated by signal")
        };
    };

    String::from_utf8(output.stdout).expect("failed to parse stdout")
}

pub fn map_lv_partition(lv_name: &str, mapped_name: &str) {
    println!("Getting storage partition");
    let storage_parted_output = command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("--machine")
            .arg(format!("/dev/data/{}", lv_name))
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
            .arg(mapped_name)
    );
}

pub fn unmap_partition(mapped_name: &str) {
    println!("Removing mapping to storage partition");
    command_stdout(
        Command::new("dmsetup")
            .arg("remove")
            .arg(mapped_name)
    );
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

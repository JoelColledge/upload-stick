use std::process::Command;
use std::str;

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

    let (from_str, to_str) = parted_find_last_free(parted_output.as_str());

    println!("Adding SD partition");
    command_stdout(
        Command::new("parted")
            .arg("--script")
            .arg("/dev/mmcblk0")
            .arg("mkpart").arg("primary").arg("").arg(from_str).arg(to_str)
    );
}

fn command_stdout(command: &mut Command) -> String {
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

fn parted_find_last_free(parted_output: &str) -> (&str, &str) {
    let free_line = parted_output.lines()
        .filter(|line| line.ends_with("free;"))
        .last()
        .expect("no 'free' lines in parted output");

    let free_line_parts: Vec<&str> = free_line.split(":").collect();

    (
        free_line_parts[1],
        free_line_parts[2]
    )
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
}

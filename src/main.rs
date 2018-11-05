use std::process::Command;
use std::str;

fn main() {
    println!("Hello, world! local repo");

    let output = Command::new("parted")
        .arg("--script")
        .arg("--machine")
        .arg("/dev/mmcblk0")
        .arg("unit").arg("MB")
        .arg("print").arg("free")
        .output()
        .expect("failed to execute process");

    println!("stderr debug: {:?}", &output.stderr);
    println!("stderr: {}", str::from_utf8(&output.stderr).expect("failed to parse stderr"));

    println!("stdout debug: {:?}", &output.stdout);
    println!("stdout: {}", str::from_utf8(&output.stdout).expect("failed to parse stdout"));
}

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

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use kohnard::driver::Driver;

include!(concat!(env!("OUT_DIR"), "/gen_tests.rs"));
include!(concat!(env!("OUT_DIR"), "/reg_tests.rs"));

fn run(cmd: &mut Command) -> (String, String, i32) {
    let output = cmd.output().expect("failed to spawn process");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

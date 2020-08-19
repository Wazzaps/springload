use std::os::unix::net::UnixStream;
use std::process::Command;
use std::process::Stdio;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;
use std::thread::sleep;
use std::time::Duration;

pub const DIARY_PATH: &str = "/diary";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let log = UnixStream::connect(DIARY_PATH).unwrap();
    let as_file1 = unsafe { std::fs::File::from_raw_fd(log.as_raw_fd()) };
    let as_file2 = unsafe { std::fs::File::from_raw_fd(log.as_raw_fd()) };
    let _cmd = Command::new(&args[0])
        .args(&args[1..])
        .stdout(Stdio::from(as_file1))
        .stderr(Stdio::from(as_file2))
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    sleep(Duration::from_millis(10));
}
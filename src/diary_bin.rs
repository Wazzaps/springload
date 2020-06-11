use std::os::unix::net::UnixStream;
use std::process::Command;
use std::process::Stdio;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;

mod diary;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let log = UnixStream::connect(diary::DIARY_PATH).unwrap();
    let as_file = unsafe { std::fs::File::from_raw_fd(log.as_raw_fd()) };
    let _cmd = Command::new(&args[0])
        .args(&args[1..])
        .stdout(Stdio::from(as_file))
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::io::AsRawFd;
use std::io::{BufReader, BufRead, Write};
use std::thread;
use nix::sys::socket::sockopt::PeerCredentials;
use nix::sys::socket::GetSockOpt;
use chrono::Local;
use nix::unistd::daemon;
use std::fs::{OpenOptions, File};
use std::sync::Mutex;
use lazy_static::lazy_static;

pub const DIARY_PATH: &str = "/diary";
lazy_static! {
    static ref OUTPUT: Mutex<Option<File>> = Mutex::new(None);
}

pub fn push_line(service_name: &str, pid: i32, line: &str) {
    // let mut output = File::open("").unwrap();
    let output = OUTPUT.lock().unwrap();
    if output.is_some() {
        output.as_ref().unwrap().write_all(format!("{} {}[{}]: {}\n", Local::now().format("%Y-%m-%d %H:%M:%S"), service_name, pid, line).as_ref()).unwrap();
    }
}

fn handle_client(stream: UnixStream) {
    let peer_fd = stream.as_raw_fd();
    let reader = BufReader::new(&stream);
    for line in reader.lines() {
        if let Ok(creds) = PeerCredentials.get(peer_fd) {
            let pid = creds.pid();
            if let Ok(line) = line {
                push_line("unknown.service", pid, &line);
            }
        }
    }
}

pub fn server() -> std::io::Result<()> {
    let listener = UnixListener::bind(DIARY_PATH)?;
    let output = OpenOptions::new().write(true).open("/dev/console").unwrap();
    OUTPUT.lock().unwrap().replace(output);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| handle_client(stream));
            }
            Err(_err) => {
                break;
            }
        }
    }

    Ok(())
}

fn main() {
    server().unwrap();
}
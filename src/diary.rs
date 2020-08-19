use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::io::AsRawFd;
use std::io::{BufReader, BufRead, Write};
use std::thread;
use nix::sys::socket::sockopt::PeerCredentials;
use nix::sys::socket::GetSockOpt;
use chrono::Local;
use std::fs::{OpenOptions, File};
use std::sync::{Mutex, RwLock, Arc, Barrier};
use lazy_static::lazy_static;
use crate::Config;

pub const DIARY_PATH: &str = "/diary";
lazy_static! {
    static ref OUTPUT: Mutex<Option<File>> = Mutex::new(None);
}

pub fn push_line(service_name: &str, pid: i32, line: &str) {
    // let mut output = File::open("").unwrap();
    let output = OUTPUT.lock().unwrap();
    let uptime = {
        let mut timespec = nix::libc::timespec { tv_sec: 0, tv_nsec: 0 };
        unsafe { nix::libc::clock_gettime(nix::libc::CLOCK_MONOTONIC_RAW, &mut timespec); }
        (timespec.tv_sec, timespec.tv_nsec / 1000000)
    };
    if output.is_some() {
        output.as_ref().unwrap().write_all(
            format!(
                "\x1b[1m{} (uptime:{}.{}) {}[{}]:\x1b[0m {}\n",
                Local::now().format("%Y-%m-%d %H:%M:%S.%3f"),
                uptime.0, uptime.1,
                service_name, pid, line
            ).as_ref()
        ).unwrap();
    }
}

fn handle_client(config: Option<Arc<RwLock<Config>>>, stream: UnixStream) {
    let peer_fd = stream.as_raw_fd();
    let reader = BufReader::new(&stream);
    let creds = PeerCredentials.get(peer_fd).unwrap();
    let mut service_name: Option<String> = None;
    for line in reader.lines() {
        let pid = creds.pid();
        if let Ok(line) = line {
            if service_name.is_none() {
                if let Some(config) = &config {
                    for (name, service) in config.read().unwrap().units.services.iter() {
                        if pid == service.root_pid {
                            service_name.replace(name.clone());
                            break
                        }
                    }
                } else {
                    service_name.replace("springload".to_string());
                }
            }

            if let Some(service_name) = &service_name {
                push_line(service_name.as_ref(), pid, &line);
            } else {
                push_line("unknown", pid, &line);
            }
        }
    }
}

pub fn server(config: Arc<RwLock<Config>>, signal: Arc<Barrier>) -> std::io::Result<()> {
    let listener = UnixListener::bind(DIARY_PATH)?;
    let output = OpenOptions::new().write(true).open("/dev/console").unwrap();
    OUTPUT.lock().unwrap().replace(output);
    signal.wait();

    let mut is_first = true;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if is_first {
                    thread::spawn(move || handle_client(None, stream));
                    is_first = false;
                } else {
                    let config_copy = config.clone();
                    thread::spawn(move || handle_client(Some(config_copy), stream));
                }
            }
            Err(_err) => {
                break;
            }
        }
    }

    Ok(())
}

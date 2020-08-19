pub use config::Config;
use crate::unit::service::ServiceType;
use nix;
use std::thread::sleep;
use std::time::Duration;
use std::collections::HashSet;
use crate::unit::{UnitState, UnitRef};
use nix::sys::wait::WaitStatus;
use std::os::unix::net::UnixStream;
use std::sync::{RwLock, Arc, Barrier};

pub mod config;
pub mod unit;
pub mod diary;

fn calculate_next<'c>(config: &'c Config, running: &HashSet<UnitRef>, target: &HashSet<UnitRef>) -> (HashSet<UnitRef>, HashSet<UnitRef>, bool) {
    let mut dep_tree = target.clone();
    let mut out_waiting = false;
    let mut more = true;
    while more {
        more = false;
        let mut next_dep_tree = dep_tree.clone();
        for unit in dep_tree.iter() {
            let unit = &config.units[unit];
            for dep_unit in unit.depends.iter().chain(unit.wants.iter()) {
                if next_dep_tree.insert(dep_unit.clone()) {
                    // new value, continue iterating
                    more = true;
                }
            }
        }
        dep_tree = next_dep_tree;
    }

    let mut to_activate = HashSet::new();
    for unit in dep_tree.iter() {
        let mut activate = true;
        for dep_unit in &config.units[unit].depends {
            if let UnitRef::Service(service) = dep_unit {
                if let Some(service) = config.units.services.get(service) {
                    if service.service_type == ServiceType::Forking && !running.contains(dep_unit) {
                        activate = false;
                        out_waiting = true;
                        break;
                    }
                }
            }
        }
        if activate {
            to_activate.insert(unit.clone());
        }
    }
    to_activate = to_activate.difference(&running).cloned().collect();
    let to_deactivate = running.difference(&dep_tree).cloned().collect();

    (to_activate, to_deactivate, out_waiting)
}

fn main() {
    // Create fs structure
    {
        std::fs::create_dir_all("/proc").unwrap();
        let mut flags = nix::mount::MsFlags::empty();
        flags.insert(nix::mount::MsFlags::MS_NOSUID);
        flags.insert(nix::mount::MsFlags::MS_NODEV);
        flags.insert(nix::mount::MsFlags::MS_NOEXEC);
        nix::mount::mount(Some("proc"), "/proc", Some("proc"), flags, None as Option<&str>).unwrap();
    }
    {
        std::fs::create_dir_all("/dev").unwrap();
        let mut flags = nix::mount::MsFlags::empty();
        flags.insert(nix::mount::MsFlags::MS_NOSUID);
        flags.insert(nix::mount::MsFlags::MS_NOEXEC);
        nix::mount::mount(Some("bold-dev"), "/dev", Some("devtmpfs"), flags, None as Option<&str>).unwrap();
    }

    std::fs::create_dir_all("/dev").unwrap();

    // Load config
    let config = Arc::new(RwLock::new(Config::from_file("/etc/springload.toml")));

    // Start diary
    {
        let diary_started = Arc::new(Barrier::new(2));
        let config_copy = config.clone();
        let diary_started_copy = diary_started.clone();
        std::thread::spawn(|| diary::server(config_copy, diary_started_copy));
        diary_started.wait();
    }

    let log = UnixStream::connect(diary::DIARY_PATH).unwrap();
    let _redirect = gag::Redirect::stdout(log).unwrap();

    // Announce ourselves
    println!("Starting springload v0.1.0");

    // Main loop
    let mut targets = HashSet::new();
    targets.insert(UnitRef::Target("interactive".to_string()));
    loop {
        {
            let mut config = config.write().unwrap();
            let running: HashSet<_> = config.units
                .iter()
                .filter(|(_, unit)| unit.state == UnitState::Started)
                .map(|(name, _)| name)
                .collect();
            let (to_activate, to_deactivate, out_waiting) = calculate_next(&config, &running, &targets);
            if !to_activate.is_empty() {
                println!("activate: {:?}", to_activate);
                for name in &to_activate {
                    if let UnitRef::Service(service) = name {
                        let service = config.units.services.get_mut(service).unwrap();
                        service.start();
                    } else if let UnitRef::Target(target) = name {
                        let target = config.units.targets.get_mut(target).unwrap();
                        target.unit.state = UnitState::Started;
                    }
                }
                if !out_waiting {
                    println!("done for now");
                }
            }
            if !to_deactivate.is_empty() {
                println!("deactivate: {:?}", to_deactivate);
            }
        }

        if let Ok(wait_res) = nix::sys::wait::wait() {
            let mut config = config.write().unwrap();
            if let WaitStatus::Exited(pid, _status) = wait_res {
                let mut found = false;
                for (_name, service) in config.units.services.iter_mut() {
                    if service.root_pid == pid.as_raw() {
                        if service.service_type == ServiceType::Forking && service.unit.state == UnitState::Starting {
                            // println!("{}'s main process forked with status: {}", name, status);
                            service.unit.state = UnitState::Started;
                        } else if service.service_type == ServiceType::Simple && service.unit.state == UnitState::Started {
                            // println!("{}'s main process exited with status: {}", name, status);
                            service.unit.state = UnitState::Stopped;
                        } else {
                            // println!("{}'s main process exited with status: {}", name, status);
                        }
                        service.root_pid = 0;
                        found = true;
                        break;
                    }
                }
                if !found {
                    // println!("pid {} exited with status: {}", pid.as_raw(), status);
                }
            } else {
                println!("{:?}", wait_res);
            }
        } else {
            sleep(Duration::from_millis(10));
        }
    }

    // Run units
    // for unit in &units {
    //     let (name, unit_type) = unit::Unit::split_name(unit);
    //     println!("[{}\x1b[90m.{}\x1b[0m]", name, unit_type);
    //     // println!("[ ] {}\x1b[90m.{}\x1b[0m", name, unit_type);
    //     // std::io::stdout().flush().unwrap();
    //
    //     match unit_type {
    //         "service" => {
    //             let svc = &config.units.services[name];
    //             let cmd: Vec<OsString> = svc.cmd.iter().map(|s| s.into()).collect();
    //
    //             match svc.isolate {
    //                 IsolateType::None => {},
    //                 _ => panic!("Unsupported isolate type"),
    //             }
    //
    //             // match svc.service_type {
    //             //     ServiceType::OneShot => {
    //             //         std::process::Command::new(&cmd[0])
    //             //             .args(&cmd[1..])
    //             //             .spawn()
    //             //             .expect(&format!("Failed to run service '{}': {:?}", unit, cmd));
    //             //     },
    //             //     _ => panic!("Unsupported service type")
    //             // }
    //         },
    //         _ => panic!("Unrunnable unit type '{}'", unit_type),
    //     }
    //
    //     // Ok
    //     // println!("\r[\x1b[92m+\x1b[0m");
    //
    //     // Error
    //     // println!("\r[\x1b[91m!\x1b[0m");
    // }
}

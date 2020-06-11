use serde::Deserialize;
use crate::unit::{Unit, UnitState};
use std::ffi::OsString;
use std::process::Stdio;
use std::fs::OpenOptions;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    /// Doesn't block at all
    Simple,

    /// Blocks until main process exits
    Forking,

    /// Blocks until all processes exit
    OneShot,

    /// Blocks until a DBus socket is available
    DBus
}

impl Default for ServiceType {
    fn default() -> Self { ServiceType::Simple }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum IsolateType {
    /// Simple spawn
    None,

    /// Spawn in cgroup
    CGroup,

    /// Spawn in bwrap
    Sandbox,
}

impl Default for IsolateType {
    fn default() -> Self { IsolateType::CGroup }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Service {
    #[serde(skip, default = "i32::default")]
    pub root_pid: i32,

    #[serde(rename = "type", default = "ServiceType::default")]
    pub service_type: ServiceType,

    #[serde(default = "IsolateType::default")]
    pub isolate: IsolateType,

    pub log: Option<String>,

    pub tty: Option<String>,

    #[serde(flatten)]
    pub unit: Unit,

    pub cmd: Vec<String>,
}

impl Service {
    pub fn start(&mut self) {
        if self.unit.state != UnitState::Stopped {
            return;
        }

        let cmd: Vec<OsString> = self.cmd.iter().map(|s| s.into()).collect();

        match self.isolate {
            IsolateType::None => {},
            _ => panic!("Unsupported isolate type"),
        }

        let stdout = if let Some(tty) = &self.tty {
            Stdio::from(OpenOptions:: new().write(true).open(tty).unwrap())
        } else if let Some(log) = &self.log {
            Stdio::from(OpenOptions:: new().write(true).open(log).unwrap())
        } else {
            Stdio::piped()
        };

        let stderr = if let Some(tty) = &self.tty {
            Stdio::from(OpenOptions:: new().write(true).open(tty).unwrap())
        } else {
            Stdio::piped()
        };

        let stdin = if let Some(tty) = &self.tty {
            Stdio::from(OpenOptions:: new().read(true).open(tty).unwrap())
        } else {
            Stdio::inherit()
        };

        let cmd = std::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .unwrap();
        self.root_pid = cmd.id() as i32;

        match self.service_type {
            ServiceType::Simple => {
                self.unit.state = UnitState::Started;
            },
            ServiceType::Forking => {
                self.unit.state = UnitState::Starting;
            },
            _ => panic!("Unsupported service type")
        }
    }
}
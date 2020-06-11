use std::collections::HashMap;

use serde::{Deserialize, Deserializer};

use crate::unit::service::Service;
use crate::unit::target::Target;
use crate::Config;
use std::ops::{Index, IndexMut};
use serde::de::Visitor;

pub mod service;
pub mod target;

#[derive(Debug, Default, Clone)]
pub struct UnitsSet {
    pub services: HashMap<String, Service>,
    pub targets: HashMap<String, Target>,
}

impl UnitsSet {
    pub fn iter(&self) -> impl Iterator<Item = (UnitRef, &Unit)> {
        let services_iter = self.services.iter()
            .map(|(name, service)| (UnitRef::Service(name.clone()), &service.unit));
        let targets_iter = self.targets.iter()
            .map(|(name, target)| (UnitRef::Target(name.clone()), &target.unit));
        services_iter.chain(targets_iter)
    }

    pub fn get(&self, index: &UnitRef) -> Option<&Unit> {
        match index {
            UnitRef::Service(name) => self.services.get(name).map(|u| &u.unit),
            UnitRef::Target(name) => self.targets.get(name).map(|u| &u.unit),
        }
    }

    pub fn get_mut(&mut self, index: &UnitRef) -> Option<&mut Unit> {
        match index {
            UnitRef::Service(name) => self.services.get_mut(name).map(|u| &mut u.unit),
            UnitRef::Target(name) => self.targets.get_mut(name).map(|u| &mut u.unit),
        }
    }
}

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub enum UnitRef {
    Service(String),
    Target(String),
}

struct UnitRefVisitor;

impl<'de> Visitor<'de> for UnitRefVisitor {
    type Value = UnitRef;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a unit name")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where E: serde::de::Error {
        if v.ends_with(".target") {
            let v = v[..v.len() - ".target".len()].to_owned();
            Ok(UnitRef::Target(v))
        } else if v.ends_with(".service") {
            let v = v[..v.len() - ".service".len()].to_owned();
            Ok(UnitRef::Service(v))
        } else {
            Err(serde::de::Error::custom("A unit name must end with .service or .target"))
        }
    }
}

impl<'de> Deserialize<'de> for UnitRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de> {
        deserializer.deserialize_string(UnitRefVisitor)
    }
}

impl Index<&UnitRef> for UnitsSet {
    type Output = Unit;

    fn index(&self, index: &UnitRef) -> &Self::Output {
        self.get(index).expect(&format!("Unit '{:?}' not found", index))
    }
}

impl IndexMut<&UnitRef> for UnitsSet {
    fn index_mut(&mut self, index: &UnitRef) -> &mut Self::Output {
        self.get_mut(index).expect(&format!("Unit '{:?}' not found", index))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnitState {
    Stopped,
    Starting,
    Started,
    Stopping,
}

impl Default for UnitState {
    fn default() -> Self { UnitState::Stopped }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Unit {
    #[serde(skip, default = "UnitState::default")]
    pub state: UnitState,

    #[serde(default = "Vec::default")]
    pub depends: Vec<UnitRef>,

    #[serde(default = "Vec::default")]
    pub wants: Vec<UnitRef>,

    pub description: Option<String>,
}

impl Unit {
    pub fn split_name(name: &str) -> (&str, &str) {
        let tmp: Vec<_> = name.rsplitn(2, ".").collect();
        (tmp[1], tmp[0])
    }

    pub fn find<'c>(unit: &str, config: &'c Config) -> Option<&'c Unit> {
        let (name, unit_type) = Unit::split_name(&unit);
        match unit_type {
            "service" => {
                Some(&config.units.services[name].unit)
            },
            "target" => {
                Some(&config.units.targets[name].unit)
            },
            _ => None,
        }
    }
}

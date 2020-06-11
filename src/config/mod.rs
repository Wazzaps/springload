use std::io::Read;
use std::path::Path;

use toml;

use crate::unit::UnitsSet;

#[derive(Debug, Default)]
pub struct Config {
    pub units: UnitsSet,
}

fn walker(source: &toml::value::Table, path: &str, set: &mut UnitsSet) {
    for (key, val) in source {
        match key.as_str() {
            "service" => {
                set.services.insert(path.to_string(), val.clone().try_into().unwrap());
            },
            "target" => {
                set.targets.insert(path.to_string(), val.clone().try_into().unwrap());
            },
            _ => {
                walker(val.as_table().unwrap(), &format!("{}.{}", path, key), set);
            }
        }

    }
}

impl Config {
    pub fn from_toml(source: &toml::Value) -> Self {
        let mut config = Config::default();
        for (key, val) in source.as_table().unwrap() {
            walker(val.as_table().unwrap(), key, &mut config.units);
        }
        config
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let mut file = std::fs::File::open(path).unwrap();
        let mut config = String::new();
        file.read_to_string(&mut config).unwrap();
        let source = toml::from_str(&config).unwrap();
        Config::from_toml(&source)
    }
}
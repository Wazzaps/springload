use serde::Deserialize;
use crate::unit::Unit;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Target {
    #[serde(flatten)]
    pub unit: Unit,
}
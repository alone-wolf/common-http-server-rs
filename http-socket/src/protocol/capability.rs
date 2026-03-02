use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityKey(String);

impl CapabilityKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CapabilityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityValue {
    Bool(bool),
    Number(i64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityRequirement {
    Present,
    BoolTrue,
    Equals(CapabilityValue),
}

impl CapabilityRequirement {
    pub fn is_satisfied_by(&self, value: &CapabilityValue) -> bool {
        match self {
            Self::Present => true,
            Self::BoolTrue => matches!(value, CapabilityValue::Bool(true)),
            Self::Equals(expected) => expected == value,
        }
    }
}

pub type CapabilityMap = BTreeMap<CapabilityKey, CapabilityValue>;
pub type CapabilitySet = BTreeSet<CapabilityKey>;
pub type CapabilityRequirementMap = BTreeMap<CapabilityKey, CapabilityRequirement>;

//! Stable opaque identity and lossless wire serialization.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use uuid::Uuid;

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
            pub fn parse(value: &str) -> Result<Self, uuid::Error> {
                value.parse().map(Self)
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
    };
}

stable_id!(ProjectId);
stable_id!(DocumentId);
stable_id!(AssetId);
stable_id!(SegmentId);
stable_id!(SegmentOrderId);
stable_id!(AlignmentId);
stable_id!(AnnotationId);
stable_id!(BookmarkId);
stable_id!(OperationId);
stable_id!(CommandId);

/// Revision IDs are strings over the wire so JavaScript cannot lose precision.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RevisionId(u64);
impl RevisionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}
impl From<u64> for RevisionId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
impl fmt::Display for RevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Serialize for RevisionId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for RevisionId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = RevisionId;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a decimal revision ID string")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value
                    .parse::<u64>()
                    .map(RevisionId)
                    .map_err(|_| E::custom("invalid revision ID"))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                self.visit_str(&value)
            }
        }
        deserializer.deserialize_string(Visitor)
    }
}

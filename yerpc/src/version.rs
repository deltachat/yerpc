//! yerpc version field
//! (c) Parity Technologies <admin@parity.io>
//! MIT License
//! https://github.com/paritytech/yerpc/blob/31ec6d67f2ab338a26f5080af5804960f7ab39e4/core/src/types/version.rs
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use typescript_type_def::{type_expr, TypeDef};

/// Protocol Version
#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq)]
pub enum Version {
    /// JSONRPC 2.0
    V2,
}

// The TypeDef macro fails to translate the enum value, so here we implement
// TypeDef manually.
impl TypeDef for Version {
    const INFO: type_expr::TypeInfo = {
        type_expr::TypeInfo::Native(type_expr::NativeTypeInfo {
            r#ref: type_expr::TypeExpr::String(type_expr::TypeString {
                value: "2.0",
                docs: None,
            }),
        })
    };
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Version::V2 => serializer.serialize_str("2.0"),
        }
    }
}

impl<'a> Deserialize<'a> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Version, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_identifier(VersionVisitor)
    }
}

struct VersionVisitor;

impl<'a> Visitor<'a> for VersionVisitor {
    type Value = Version;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "2.0" => Ok(Version::V2),
            _ => Err(de::Error::custom("invalid version")),
        }
    }
}

//! Primitive, validated value types shared across every `cloud-forge`
//! crate: [`ResourceId`], [`ResourceType`], [`AccountId`], [`RegionId`],
//! [`AzId`], [`Arn`], and [`Timestamp`]. Every constructor validates
//! its input and returns a [`cloud_errors::CloudError`] on failure --
//! nothing here silently truncates, coerces, or accepts a malformed
//! value.
#![forbid(unsafe_code)]

mod arn;
mod id;
mod region;
mod timestamp;

pub use arn::Arn;
pub use id::{AccountId, ResourceId, ResourceType};
pub use region::{AzId, RegionId};
pub use timestamp::Timestamp;

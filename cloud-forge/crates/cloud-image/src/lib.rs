// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A registry of machine images a compute primitive can be launched
//! from.
//!
//! Images are immutable once registered: there is no update method,
//! only `register`/`get`/`list`. An image whose bytes could change
//! after something already referenced its id is not the kind of
//! primitive a resource model should be built on. Whether an in-use
//! image can later be deregistered depends on resource-to-image
//! references this crate has no way to see on its own -- that
//! composition, and deregistration itself, are left to whatever later
//! phase actually launches instances from a registered image.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::ResourceId;
use std::collections::BTreeMap;

/// The CPU architecture a registered image was built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    Arm64,
}

/// A registered machine image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    id: ResourceId,
    name: String,
    size_bytes: u64,
    architecture: Architecture,
}

impl Image {
    pub fn id(&self) -> &ResourceId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn architecture(&self) -> Architecture {
        self.architecture
    }
}

/// A registry of machine images, keyed by [`ResourceId`].
#[derive(Debug, Clone, Default)]
pub struct ImageRegistry {
    images: BTreeMap<ResourceId, Image>,
}

impl ImageRegistry {
    pub fn new() -> Self {
        ImageRegistry {
            images: BTreeMap::new(),
        }
    }

    /// Registers a new image. Rejects a second registration of an
    /// already-used `id` and a zero-byte size.
    pub fn register(
        &mut self,
        id: ResourceId,
        name: impl Into<String>,
        size_bytes: u64,
        architecture: Architecture,
    ) -> Result<&Image, CloudError> {
        if self.images.contains_key(&id) {
            return Err(CloudError::Conflict {
                what: "image",
                id: id.to_string(),
            });
        }
        if size_bytes == 0 {
            return Err(CloudError::InvalidFormat {
                what: "image size",
                value: size_bytes.to_string(),
                reason: "must be greater than zero".to_string(),
            });
        }
        self.images.insert(
            id.clone(),
            Image {
                id: id.clone(),
                name: name.into(),
                size_bytes,
                architecture,
            },
        );
        Ok(self
            .images
            .get(&id)
            .expect("just inserted under this exact id"))
    }

    pub fn get(&self, id: &ResourceId) -> Option<&Image> {
        self.images.get(id)
    }

    pub fn contains(&self, id: &ResourceId) -> bool {
        self.images.contains_key(id)
    }

    /// Registered images, in deterministic (sorted-by-id) order.
    pub fn list(&self) -> impl Iterator<Item = &Image> {
        self.images.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> ResourceId {
        ResourceId::new(s).unwrap()
    }

    #[test]
    fn register_then_get_returns_the_image() {
        let mut reg = ImageRegistry::new();
        reg.register(
            id("ami-1"),
            "base-linux",
            2_147_483_648,
            Architecture::X86_64,
        )
        .unwrap();
        let image = reg.get(&id("ami-1")).unwrap();
        assert_eq!(image.id(), &id("ami-1"));
        assert_eq!(image.name(), "base-linux");
        assert_eq!(image.size_bytes(), 2_147_483_648);
        assert_eq!(image.architecture(), Architecture::X86_64);
    }

    #[test]
    fn registering_a_duplicate_id_is_a_conflict() {
        let mut reg = ImageRegistry::new();
        reg.register(id("ami-1"), "base-linux", 1024, Architecture::X86_64)
            .unwrap();
        let err = reg
            .register(id("ami-1"), "other-name", 2048, Architecture::Arm64)
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
        // The original registration must be untouched.
        assert_eq!(reg.get(&id("ami-1")).unwrap().name(), "base-linux");
    }

    #[test]
    fn registering_a_zero_byte_image_is_rejected() {
        let mut reg = ImageRegistry::new();
        let err = reg
            .register(id("ami-1"), "empty", 0, Architecture::X86_64)
            .unwrap_err();
        assert!(matches!(err, CloudError::InvalidFormat { .. }));
        assert!(!reg.contains(&id("ami-1")));
    }

    #[test]
    fn get_of_an_unregistered_id_returns_none() {
        let reg = ImageRegistry::new();
        assert_eq!(reg.get(&id("ami-1")), None);
    }

    #[test]
    fn contains_reflects_registration_state() {
        let mut reg = ImageRegistry::new();
        assert!(!reg.contains(&id("ami-1")));
        reg.register(id("ami-1"), "base-linux", 1024, Architecture::X86_64)
            .unwrap();
        assert!(reg.contains(&id("ami-1")));
    }

    #[test]
    fn list_includes_all_registered_images_in_sorted_id_order() {
        let mut reg = ImageRegistry::new();
        reg.register(id("ami-2"), "second", 1024, Architecture::Arm64)
            .unwrap();
        reg.register(id("ami-1"), "first", 1024, Architecture::X86_64)
            .unwrap();
        let ids: Vec<&ResourceId> = reg.list().map(Image::id).collect();
        assert_eq!(ids, vec![&id("ami-1"), &id("ami-2")]);
    }

    #[test]
    fn two_different_images_can_share_the_same_name() {
        let mut reg = ImageRegistry::new();
        reg.register(id("ami-1"), "base-linux", 1024, Architecture::X86_64)
            .unwrap();
        reg.register(id("ami-2"), "base-linux", 2048, Architecture::Arm64)
            .unwrap();
        assert_eq!(reg.list().count(), 2);
    }
}

// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! An [`Account`] (a validated [`AccountId`] plus a display name) and
//! an [`AccountRegistry`] tracking which account ids are already in
//! use.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::{AccountId, Timestamp};
use std::collections::BTreeMap;

const MAX_NAME_LEN: usize = 256;

/// A cloud account: an id and a human-readable name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    name: String,
    created_at: Timestamp,
}

impl Account {
    pub fn new(
        id: AccountId,
        name: impl Into<String>,
        created_at: Timestamp,
    ) -> Result<Self, CloudError> {
        let name = name.into();
        if name.is_empty() {
            return Err(CloudError::InvalidFormat {
                what: "account name",
                value: name,
                reason: "must not be empty".to_string(),
            });
        }
        if name.chars().count() > MAX_NAME_LEN {
            return Err(CloudError::InvalidFormat {
                what: "account name",
                value: name,
                reason: format!("must be at most {MAX_NAME_LEN} characters"),
            });
        }
        if name.chars().any(|c| c.is_control()) {
            return Err(CloudError::InvalidFormat {
                what: "account name",
                value: name,
                reason: "must not contain control characters".to_string(),
            });
        }
        Ok(Account {
            id,
            name,
            created_at,
        })
    }

    pub fn id(&self) -> &AccountId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }
}

/// Tracks which [`AccountId`]s have already been registered, so a
/// second registration under the same id is a caught conflict rather
/// than a silent overwrite.
#[derive(Debug, Clone, Default)]
pub struct AccountRegistry {
    accounts: BTreeMap<AccountId, Account>,
}

impl AccountRegistry {
    pub fn new() -> Self {
        AccountRegistry {
            accounts: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, account: Account) -> Result<(), CloudError> {
        if self.accounts.contains_key(&account.id) {
            return Err(CloudError::Conflict {
                what: "account",
                id: account.id.to_string(),
            });
        }
        self.accounts.insert(account.id.clone(), account);
        Ok(())
    }

    pub fn get(&self, id: &AccountId) -> Option<&Account> {
        self.accounts.get(id)
    }

    pub fn contains(&self, id: &AccountId) -> bool {
        self.accounts.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account_id() -> AccountId {
        AccountId::new("000000000001").unwrap()
    }

    #[test]
    fn new_account_accepts_a_valid_name() {
        let a = Account::new(account_id(), "prod-account", Timestamp::EPOCH).unwrap();
        assert_eq!(a.name(), "prod-account");
    }

    #[test]
    fn rejects_empty_or_too_long_name() {
        assert!(Account::new(account_id(), "", Timestamp::EPOCH).is_err());
        let long = "x".repeat(MAX_NAME_LEN + 1);
        assert!(Account::new(account_id(), long, Timestamp::EPOCH).is_err());
    }

    #[test]
    fn rejects_control_characters_in_name() {
        assert!(Account::new(account_id(), "bad\nname", Timestamp::EPOCH).is_err());
    }

    #[test]
    fn registry_rejects_duplicate_registration() {
        let mut reg = AccountRegistry::new();
        let a = Account::new(account_id(), "first", Timestamp::EPOCH).unwrap();
        reg.register(a).unwrap();

        let dup = Account::new(account_id(), "second", Timestamp::EPOCH).unwrap();
        assert!(matches!(
            reg.register(dup),
            Err(CloudError::Conflict { .. })
        ));
        // The original registration must be untouched.
        assert_eq!(reg.get(&account_id()).unwrap().name(), "first");
    }

    #[test]
    fn contains_reflects_registration_state() {
        let mut reg = AccountRegistry::new();
        assert!(!reg.contains(&account_id()));
        reg.register(Account::new(account_id(), "a", Timestamp::EPOCH).unwrap())
            .unwrap();
        assert!(reg.contains(&account_id()));
    }
}

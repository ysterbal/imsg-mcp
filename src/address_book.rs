use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::ImsgError;

#[derive(Debug, Clone)]
pub struct Contact {
    pub given_name: String,
    pub family_name: String,
    #[allow(dead_code)]
    pub phone_numbers: Vec<String>,
}

impl Contact {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.given_name, self.family_name).trim().to_string()
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
#[allow(dead_code)]
pub struct AddressBook {
    store: Option<()>,
    contacts_cache: HashMap<String, Contact>,
    last_cache_update: u64,
    cache_ttl: u64,
}

#[cfg(target_os = "macos")]
impl AddressBook {
    pub fn new() -> Result<Self, ImsgError> {
        // Contacts access requires proper objc2 bindings and entitlements
        // For now, return a basic AddressBook with empty cache
        Ok(Self {
            store: None,
            contacts_cache: HashMap::new(),
            last_cache_update: 0,
            cache_ttl: 300,
        })
    }

    #[allow(dead_code)]
    fn update_cache_if_needed(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if current_time - self.last_cache_update > self.cache_ttl {
            // In a full implementation, this would fetch from CNContactStore
            self.last_cache_update = current_time;
        }
    }

    pub fn get_contact_by_phone(&self, _phone_number: &str) -> Option<Contact> {
        // Return None for now as full CNContactStore integration requires entitlements
        None
    }
}

#[cfg(not(target_os = "macos"))]
pub struct AddressBook;

#[cfg(not(target_os = "macos"))]
impl AddressBook {
    pub fn new() -> Result<Self, ImsgError> {
        Err(ImsgError::PlatformNotSupported)
    }

    pub fn get_contact_by_phone(&mut self, _phone_number: &str) -> Option<Contact> {
        None
    }
}

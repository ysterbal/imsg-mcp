use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::ImsgError;
use rusqlite::Connection;

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

fn normalize_phone_number(phone: &str) -> String {
    phone.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn get_contact_from_address_book_db(phone_number: &str) -> Option<Contact> {
    let normalized_input = normalize_phone_number(phone_number);
    if normalized_input.is_empty() {
        return None;
    }

    // Find AddressBook database
    let home = std::env::var("HOME").ok()?;
    let ab_dir = Path::new(&home).join("Library/Application Support/AddressBook/Sources");
    
    if !ab_dir.exists() {
        return None;
    }

    // Find AddressBook-v*.abcddb files
    let mut db_path: Option<String> = None;
    if let Ok(entries) = fs::read_dir(&ab_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                for db_file_name in [
                    "AddressBook-v22.abcddb",
                    "AddressBook-v21.abcddb",
                    "AddressBook-v20.abcddb",
                    "AddressBook-v19.abcddb",
                    "AddressBook-v18.abcddb",
                    "AddressBook-v17.abcddb",
                ] {
                    let db_file = path.join(db_file_name);
                    if db_file.exists() {
                        db_path = Some(db_file.to_string_lossy().to_string());
                        break;
                    }
                }
                if db_path.is_some() {
                    break;
                }
            }
        }
    }

    let db_path = db_path?;
    
    // Connect to the database and query
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    // Query to get contact name and phone number
    let query = r#"
        SELECT r.ZFIRSTNAME, r.ZLASTNAME, p.ZFULLNUMBER
        FROM ZABCDRECORD r
        JOIN ZABCDPHONENUMBER p ON r.Z_PK = p.ZOWNER
        WHERE p.ZFULLNUMBER IS NOT NULL
    "#;

    let mut stmt = match conn.prepare(query) {
        Ok(s) => s,
        Err(_) => return None,
    };

    let rows = match stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    }) {
        Ok(r) => r,
        Err(_) => return None,
    };

    for row_result in rows {
        let (first_name_opt, last_name_opt, full_number_opt) = match row_result {
            Ok(val) => val,
            Err(_) => continue,
        };

        let first_name = first_name_opt.unwrap_or_default();
        let last_name = last_name_opt.unwrap_or_default();
        let full_number = full_number_opt.unwrap_or_default();

        // Normalize the full number
        let normalized_db_number = normalize_phone_number(&full_number);
        
        // Check if the normalized_db_number matches or contains the normalized_input
        if normalized_db_number == normalized_input {
            return Some(Contact {
                given_name: first_name,
                family_name: last_name,
                phone_numbers: vec![],
            });
        }
        
        // If the input is shorter, check if it's a suffix match (e.g., last 7-10 digits)
        if normalized_input.len() < normalized_db_number.len()
            && normalized_db_number.ends_with(&normalized_input)
        {
            return Some(Contact {
                given_name: first_name,
                family_name: last_name,
                phone_numbers: vec![],
            });
        }
    }

    None
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AddressBook {
    contacts_cache: HashMap<String, Contact>,
    last_cache_update: u64,
    cache_ttl: u64,
}

#[cfg(target_os = "macos")]
impl AddressBook {
    pub fn new() -> Result<Self, ImsgError> {
        Ok(Self {
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
            self.last_cache_update = current_time;
        }
    }

    pub fn get_contact_by_phone(&self, phone_number: &str) -> Option<Contact> {
        // Query the local AddressBook SQLite database directly
        get_contact_from_address_book_db(phone_number)
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

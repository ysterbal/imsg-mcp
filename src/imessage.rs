use chrono::Utc;
use rusqlite::{Connection, Result as SqliteResult};

use crate::errors::{ImsgError, Result};
use crate::models::MessageDTO;
use crate::snowflake::SnowflakeDecoder;

#[cfg(target_os = "macos")]
use crate::address_book::AddressBook;

#[derive(Debug)]
pub struct IMessageServer {
    db_location: String,
    #[allow(dead_code)]
    address_book: Option<AddressBook>,
}

impl IMessageServer {
    pub fn new(db_location: Option<String>, address_book: Option<AddressBook>) -> Self {
        Self {
            db_location: db_location.unwrap_or_else(|| "~/Library/Messages/chat.db".to_string()),
            address_book,
        }
    }

    fn get_db_connection(&self) -> SqliteResult<Connection> {
        let db_path = shellexpand::tilde(&self.db_location).to_string();
        Connection::open(db_path)
    }

    pub fn get_received_messages(&self, limit: usize) -> Result<Vec<MessageDTO>> {
        let conn = self.get_db_connection().map_err(ImsgError::DatabaseError)?;

        let mut stmt = conn.prepare(
            r#"
            SELECT 
                m.ROWID,
                m.date,
                m.text,
                m.attributedBody,
                m.is_from_me,
                m.cache_roomnames,
                h.id
            FROM message m
            LEFT OUTER JOIN handle h ON m.handle_id = h.ROWID
            WHERE m.is_from_me = 0
            ORDER BY m.date DESC
            LIMIT ?
            "#,
        )?;

        let messages: Vec<MessageDTO> = stmt
            .query_map([limit as i64], |row| {
                let rowid: i64 = row.get(0)?;
                let date: i64 = row.get(1)?;
                let text: Option<String> = row.get(2)?;
                let attributed_body: Option<Vec<u8>> = row.get(3)?;
                let is_from_me: i32 = row.get(4)?;
                let cache_roomnames: Option<String> = row.get(5)?;
                let handle_id: Option<String> = row.get(6)?;

                let datetime_val = SnowflakeDecoder::decode(date).map(|c| c.datetime_utc).unwrap_or_else(|_| Utc::now());

                let phone_number = handle_id.unwrap_or_else(|| "Unknown".to_string());

                let full_name = if let Some(ab) = &self.address_book {
                    if is_from_me == 0 && phone_number != "Me" && phone_number != "Unknown" {
                        ab.get_contact_by_phone(&phone_number).map(|c| c.full_name())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let body = Self::process_message_body(text.as_deref(), attributed_body.as_deref());

                let cache_roomname = cache_roomnames.unwrap_or_default();

                Ok(MessageDTO {
                    rowid,
                    datetime: datetime_val,
                    body: body.unwrap_or_default(),
                    phone_number: phone_number.clone(),
                    is_from_me: is_from_me != 0,
                    cache_roomname: cache_roomname.clone(),
                    group_chat_name: None,
                    full_name,
                })
            })?
            .filter(|msg| match msg {
                Ok(m) => !m.body.is_empty(),
                Err(_) => false,
            })
            .map(|msg| msg.unwrap())
            .collect();

        Ok(messages)
    }

    pub fn get_sent_messages(&self, limit: usize) -> Result<Vec<MessageDTO>> {
        let conn = self.get_db_connection().map_err(ImsgError::DatabaseError)?;

        let mut stmt = conn.prepare(
            r#"
            SELECT 
                m.ROWID,
                m.date,
                m.text,
                m.attributedBody,
                m.is_from_me,
                m.cache_roomnames,
                h.id
            FROM message m
            LEFT OUTER JOIN handle h ON m.handle_id = h.ROWID
            WHERE m.is_from_me = 1
            ORDER BY m.date DESC
            LIMIT ?
            "#,
        )?;

        let messages: Vec<MessageDTO> = stmt
            .query_map([limit as i64], |row| {
                let rowid: i64 = row.get(0)?;
                let date: i64 = row.get(1)?;
                let text: Option<String> = row.get(2)?;
                let attributed_body: Option<Vec<u8>> = row.get(3)?;
                let is_from_me: i32 = row.get(4)?;
                let cache_roomnames: Option<String> = row.get(5)?;
                let handle_id: Option<String> = row.get(6)?;

                let datetime_val = SnowflakeDecoder::decode(date).map(|c| c.datetime_utc).unwrap_or_else(|_| Utc::now());

                let phone_number = handle_id.unwrap_or_else(|| "Me".to_string());

                let body = Self::process_message_body(text.as_deref(), attributed_body.as_deref());

                let cache_roomname = cache_roomnames.unwrap_or_default();

                Ok(MessageDTO {
                    rowid,
                    datetime: datetime_val,
                    body: body.unwrap_or_default(),
                    phone_number: phone_number.clone(),
                    is_from_me: is_from_me != 0,
                    cache_roomname: cache_roomname.clone(),
                    group_chat_name: None,
                    full_name: if is_from_me != 0 { Some("Me".to_string()) } else { None },
                })
            })?
            .filter(|msg| match msg {
                Ok(m) => !m.body.is_empty(),
                Err(_) => false,
            })
            .map(|msg| msg.unwrap())
            .collect();

        Ok(messages)
    }

    fn process_message_body(text: Option<&str>, attributed_body: Option<&[u8]>) -> Option<String> {
        if let Some(t) = text {
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }

        if let Some(ab) = attributed_body {
            let decoded_text = String::from_utf8_lossy(ab).to_string();
            if decoded_text.contains("NSNumber") {
                let mut decoded_text = decoded_text.split("NSNumber").next().unwrap_or("").to_string();
                if decoded_text.contains("NSString") {
                    decoded_text = decoded_text.split("NSString").last().unwrap_or("").to_string();
                    if decoded_text.contains("NSDictionary") {
                        decoded_text = decoded_text.split("NSDictionary").next().unwrap_or("").to_string();
                        decoded_text = decoded_text[6..decoded_text.len().saturating_sub(12)].to_string();
                        return Some(decoded_text);
                    }
                }
                return Some(decoded_text);
            }
            return Some(decoded_text.to_string());
        }

        None
    }
}

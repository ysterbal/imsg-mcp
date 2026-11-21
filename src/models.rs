use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MessageDTO {
    pub rowid: i64,
    pub datetime: DateTime<Utc>,
    pub body: String,
    pub phone_number: String,
    pub is_from_me: bool,
    pub cache_roomname: String,
    pub group_chat_name: Option<String>,
    pub full_name: Option<String>,
}

impl MessageDTO {
    pub fn to_string(&self) -> String {
        let direction = if self.is_from_me { "→" } else { "←" };
        let sender = self.full_name.as_ref().unwrap_or(&self.phone_number);
        let date_str = self.datetime.format("%H:%M").to_string();
        let body = if self.body.is_empty() { "(No message content)" } else { &self.body };
        
        format!("[{} {} {}]: {}", date_str, direction, sender, body)
    }
}

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SnowflakeComponents {
    pub timestamp_ms: i64,
    pub sequence: i32,
    pub worker_id: i32,
    pub process_id: i32,
    pub raw_timestamp: i64,
    pub datetime_utc: DateTime<Utc>,
}

pub struct SnowflakeDecoder;

impl SnowflakeDecoder {
    // Epoch (January 1, 2001 00:00:00.000 UTC)
    const EPOCH: i64 = 978307200000;

    // Each timestamp unit represents 2^22 nanoseconds
    const NANOS_PER_UNIT: i64 = 1 << 22; // 2^22 nanoseconds
    const NANOS_PER_MS: i64 = 1_000_000; // 10^6 nanoseconds

    // Bit lengths
    const TIMESTAMP_BITS: u32 = 42;
    const WORKER_BITS: u32 = 10;
    const PROCESS_BITS: u32 = 5;
    const SEQUENCE_BITS: u32 = 7;

    // Bit masks
    const TIMESTAMP_MASK: i64 = (1 << Self::TIMESTAMP_BITS) - 1;
    const WORKER_MASK: i64 = (1 << Self::WORKER_BITS) - 1;
    const PROCESS_MASK: i64 = (1 << Self::PROCESS_BITS) - 1;
    const SEQUENCE_MASK: i64 = (1 << Self::SEQUENCE_BITS) - 1;

    pub fn decode(snowflake_id: i64) -> Result<SnowflakeComponents, super::errors::ImsgError> {
        // Extract components using bit operations
        let raw_timestamp = (snowflake_id >> (Self::WORKER_BITS + Self::PROCESS_BITS + Self::SEQUENCE_BITS)) & Self::TIMESTAMP_MASK;
        let worker_id = (snowflake_id >> (Self::PROCESS_BITS + Self::SEQUENCE_BITS)) & Self::WORKER_MASK;
        let process_id = (snowflake_id >> Self::SEQUENCE_BITS) & Self::PROCESS_MASK;
        let sequence = snowflake_id & Self::SEQUENCE_MASK;

        // Convert to milliseconds using exact nanosecond math
        let ms_since_epoch = (raw_timestamp * Self::NANOS_PER_UNIT) / Self::NANOS_PER_MS;
        let timestamp_ms = Self::EPOCH + ms_since_epoch;

        // Convert to datetime objects
        let dt_utc = Utc.timestamp_millis_opt(timestamp_ms).single().ok_or_else(|| {
            super::errors::ImsgError::InvalidSnowflake(format!("Invalid timestamp: {}", timestamp_ms))
        })?;

        Ok(SnowflakeComponents {
            timestamp_ms,
            sequence: sequence as i32,
            worker_id: worker_id as i32,
            process_id: process_id as i32,
            raw_timestamp,
            datetime_utc: dt_utc,
        })
    }
}

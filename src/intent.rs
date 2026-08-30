//! Courtesy Context Call — not a token, not a score.
//!
//! 30 days default after last 6W ☑. 90 if they set it. Nothing further.
//! Message when due (and score is already 100): "Time to check your Context."
//! Check resets the clock. Ignore allowed; Mk4 does not move.

use serde_yaml_ng::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_INTERVAL_DAYS: u32 = 30;
pub const MAX_INTERVAL_DAYS: u32 = 90;
pub const COURTESY_LINE: &str = "Time to check your Context.";

#[derive(Debug, Clone)]
pub struct ContextCheck {
    pub last_at_unix: u64,
    pub interval_days: u32,
}

impl ContextCheck {
    pub fn stamp(interval_days: u32) -> Self {
        Self {
            last_at_unix: now_unix(),
            interval_days: normalize_interval(interval_days),
        }
    }

    pub fn is_due(&self) -> bool {
        let now = now_unix();
        let ttl_secs = u64::from(self.interval_days) * 24 * 60 * 60;
        now.saturating_sub(self.last_at_unix) >= ttl_secs
    }

    pub fn last_at_rfc3339(&self) -> String {
        unix_to_rfc3339(self.last_at_unix)
    }
}

pub fn normalize_interval(days: u32) -> u32 {
    if days == MAX_INTERVAL_DAYS {
        MAX_INTERVAL_DAYS
    } else {
        DEFAULT_INTERVAL_DAYS
    }
}

pub fn read(doc: &Value) -> Option<ContextCheck> {
    let block = doc.get("context_check")?;
    let interval = block
        .get("interval_days")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_INTERVAL_DAYS as u64) as u32;
    if let Some(n) = block.get("last_at_unix").and_then(|v| v.as_u64()) {
        return Some(ContextCheck {
            last_at_unix: n,
            interval_days: normalize_interval(interval),
        });
    }
    let s = block.get("last_at")?.as_str()?;
    let unix = parse_rfc3339_unix(s)?;
    Some(ContextCheck {
        last_at_unix: unix,
        interval_days: normalize_interval(interval),
    })
}

pub fn write(doc: &mut Value, check: &ContextCheck) {
    let mapping = match doc {
        Value::Mapping(m) => m,
        _ => return,
    };
    let mut block = serde_yaml_ng::Mapping::new();
    block.insert(
        Value::String("last_at".into()),
        Value::String(check.last_at_rfc3339()),
    );
    block.insert(
        Value::String("last_at_unix".into()),
        Value::Number(check.last_at_unix.into()),
    );
    block.insert(
        Value::String("interval_days".into()),
        Value::Number(check.interval_days.into()),
    );
    mapping.insert(Value::String("context_check".into()), Value::Mapping(block));
}

/// Courtesy line only when occupancy is already 100 and the clock is due.
/// Below 100 is HITL (`faf_go`), not maintenance.
pub fn courtesy_line(doc: &Value, score: u32) -> Option<&'static str> {
    if score < 100 {
        return None;
    }
    let check = read(doc)?;
    if check.is_due() {
        Some(COURTESY_LINE)
    } else {
        None
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn unix_to_rfc3339(secs: u64) -> String {
    let days = secs / 86400;
    let rem = secs % 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn parse_rfc3339_unix(s: &str) -> Option<u64> {
    let s = s.trim().trim_end_matches('Z');
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-');
    let y: i32 = d.next()?.parse().ok()?;
    let m: u32 = d.next()?.parse().ok()?;
    let day: u32 = d.next()?.parse().ok()?;
    let mut t = time.split(':');
    let h: u64 = t.next()?.parse().ok()?;
    let min: u64 = t.next()?.parse().ok()?;
    let sec: u64 = t.next()?.parse::<f64>().ok()? as u64;
    let days = days_from_civil(y, m, day)?;
    Some(days * 86400 + h * 3600 + min * 60 + sec)
}

fn days_from_civil(y: i32, m: u32, d: u32) -> Option<u64> {
    if m < 1 || m > 12 || d < 1 || d > 31 {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m as u64 - 3 } else { m as u64 + 9 };
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let z = era * 146097 + doe as i64 - 719468;
    if z < 0 {
        None
    } else {
        Some(z as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_only_30_or_90() {
        assert_eq!(normalize_interval(30), 30);
        assert_eq!(normalize_interval(90), 90);
        assert_eq!(normalize_interval(7), 30);
        assert_eq!(normalize_interval(180), 30);
    }

    #[test]
    fn fresh_check_not_due() {
        let c = ContextCheck::stamp(30);
        assert!(!c.is_due());
        let mut v = Value::Mapping(serde_yaml_ng::Mapping::new());
        write(&mut v, &c);
        assert!(courtesy_line(&v, 100).is_none());
        assert!(courtesy_line(&v, 70).is_none());
    }

    #[test]
    fn below_100_never_courtesy() {
        let mut c = ContextCheck::stamp(30);
        c.last_at_unix = 1;
        let mut v = Value::Mapping(serde_yaml_ng::Mapping::new());
        write(&mut v, &c);
        assert!(courtesy_line(&v, 70).is_none());
        assert_eq!(courtesy_line(&v, 100), Some(COURTESY_LINE));
    }

    #[test]
    fn rfc3339_roundtrip_unix() {
        let u = 1_777_000_000;
        let s = unix_to_rfc3339(u);
        let back = parse_rfc3339_unix(&s).unwrap();
        assert_eq!(back, u);
    }
}

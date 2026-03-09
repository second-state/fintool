use colored::Colorize;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Format f64 to string with 8 decimal places, then strip trailing zeros.
/// Matches Hyperliquid SDK's float_to_string_for_hashing convention.
pub fn fmt_num(val: f64) -> String {
    let s = format!("{:.8}", val);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn color_change(value: &str) -> String {
    match Decimal::from_str(value) {
        Ok(d) if d > Decimal::ZERO => format!("+{}%", value).green().to_string(),
        Ok(d) if d < Decimal::ZERO => format!("{}%", value).red().to_string(),
        _ => format!("{}%", value).to_string(),
    }
}

pub fn color_pnl(value: &str) -> String {
    match Decimal::from_str(value) {
        Ok(d) if d > Decimal::ZERO => format!("+{}", value).green().to_string(),
        Ok(d) if d < Decimal::ZERO => value.red().to_string(),
        _ => value.to_string(),
    }
}

pub fn time_ago(timestamp: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(dt);
        if duration.num_hours() > 24 {
            format!("{}d ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{}h ago", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{}m ago", duration.num_minutes())
        } else {
            "just now".to_string()
        }
    } else {
        timestamp.to_string()
    }
}

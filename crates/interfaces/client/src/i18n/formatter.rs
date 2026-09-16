//! Human-readable value formatting shared by metric and billing views.

pub fn currency(value: f64) -> String {
    format!("${value:.2}")
}

pub fn integer(value: u64) -> String {
    format_with_separators(value)
}

pub fn tokens(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.2}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn format_with_separators(value: u64) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, ch) in raw.chars().enumerate() {
        if index > 0 && (raw.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

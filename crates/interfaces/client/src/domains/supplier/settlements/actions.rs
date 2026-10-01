pub fn format_currency(value: f64) -> String {
    let raw = format!("{value:.2}");
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw.as_str(), "00"));
    let mut grouped_reversed = String::new();
    for (index, character) in whole.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped_reversed.push(',');
        }
        grouped_reversed.push(character);
    }
    let grouped: String = grouped_reversed.chars().rev().collect();
    format!("${grouped}.{fraction}")
}

pub fn progress_width(percent: u8) -> String {
    format!("width: {}%", percent.min(100))
}

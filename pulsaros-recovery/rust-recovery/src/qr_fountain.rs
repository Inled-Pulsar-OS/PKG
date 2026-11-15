pub const FOUNTAIN_PLAYER_HTML: &str = include_str!("fountain_player.html");

/// Generates a self-contained HTML page containing the Luby Transform (LT) Fountain QR player
/// compatible with https://qrs.inled.es.
pub fn generate_fountain_html(log_data: &str, file_name: &str) -> String {
    let log_json = serde_json::to_string(log_data).unwrap_or_else(|_| "\"\"".to_string());
    let file_json = serde_json::to_string(file_name).unwrap_or_else(|_| "\"pulsaros-recovery.log\"".to_string());
    FOUNTAIN_PLAYER_HTML
        .replace("__LOG_DATA_JSON__", &log_json)
        .replace("__FILE_NAME_JSON__", &file_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_fountain_html() {
        let sample = "2026-11-15 test log contents\nline 2";
        let html = generate_fountain_html(sample, "test.log");
        assert!(html.contains("qrs.inled.es"));
        assert!(html.contains("test.log"));
        assert!(html.contains("QRFountain"));
    }
}

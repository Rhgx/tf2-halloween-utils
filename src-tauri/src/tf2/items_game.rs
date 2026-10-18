//! What the app reads from TF2's items_game.txt at run time.

use std::path::Path;

/// When the newest Scream Fortress's contracts end, as "YYYY-MM-DD", from the game's items_game.txt (its
/// `operation_halloween<year>` entry). Valve moves the date there when it extends the event.
pub fn contracts_end(tf_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(tf_dir.join("tf/scripts/items/items_game.txt")).ok()?;
    contracts_end_in(&String::from_utf8_lossy(&bytes))
}

fn contracts_end_in(items_game: &str) -> Option<String> {
    let mut name = "";
    let mut newest: Option<(&str, &str)> = None;
    for line in items_game.lines() {
        // the quoted tokens of a `"key" "value"` line
        let mut quoted = line.split('"').skip(1).step_by(2);
        match (quoted.next(), quoted.next()) {
            (Some("name"), Some(value)) => name = value,
            // same-length names, so the later year sorts last
            (Some("contracts_end_date"), Some(date))
                if name.starts_with("operation_halloween")
                    && newest.is_none_or(|(seen, _)| name > seen) =>
            {
                newest = Some((name, date));
            }
            _ => {}
        }
    }
    newest
        .and_then(|(_, date)| date.get(..10))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_halloween_operation_wins() {
        let text = r#"
            "17"
            {
                "name"                  "operation_halloween2026"
                "contracts_end_date"    "2026-11-08 00:00:00"
            }
            "16"
            {
                "name"                  "operation_halloween2025"
                "contracts_end_date"    "2025-11-08 00:00:00"
            }
            "18"
            {
                "name"                  "operation_summer2027"
                "contracts_end_date"    "2027-08-01 00:00:00"
            }
        "#;
        assert_eq!(contracts_end_in(text).as_deref(), Some("2026-11-08"));
        assert_eq!(contracts_end_in(""), None);
    }
}

use serde_json;
use serde::de;

pub fn parse_jsonc<'a, T: de::DeserializeOwned>(s: &str) -> serde_json::Result<T> {
    // TODO: would be nice to also handle /* block comments */
    let mut stripped = String::new();
    for line in s.lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        stripped.push_str(line);
    }
    serde_json::from_str(stripped.as_str())
}
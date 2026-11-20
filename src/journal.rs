use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::domain::body::Body;

type BodyKey = (String, String);

#[derive(Debug, Clone)]
struct FootfallRecord {
    was_footfalled: bool,
    timestamp: Option<String>,
}

#[derive(Debug, Default)]
pub struct JournalFootfallReport {
    pub directory: Option<PathBuf>,
    pub files_scanned: usize,
    pub bodies_matched: usize,
}

/// Read Scan.WasFootfalled from local Elite Dangerous journals and attach the
/// latest known value to matching search results. Spansh has no direct field.
pub fn apply_local_journal_footfall(bodies: &mut [Body]) -> JournalFootfallReport {
    let Some(directory) = discover_journal_directory() else {
        return JournalFootfallReport::default();
    };
    apply_journal_footfall_from_directory(bodies, &directory)
}

fn discover_journal_directory() -> Option<PathBuf> {
    if let Some(override_path) = env::var_os("ED_JOURNAL_DIR") {
        let path = PathBuf::from(override_path);
        return path.is_dir().then_some(path);
    }

    let home = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)?;

    let candidates = [
        home.join("Saved Games/Frontier Developments/Elite Dangerous"),
        home.join(".local/share/Frontier Developments/Elite Dangerous"),
        home.join("Library/Application Support/Frontier Developments/Elite Dangerous"),
        home.join(".steam/steam/steamapps/compatdata/359320/pfx/drive_c/users/steamuser/Saved Games/Frontier Developments/Elite Dangerous"),
        home.join(".local/share/Steam/steamapps/compatdata/359320/pfx/drive_c/users/steamuser/Saved Games/Frontier Developments/Elite Dangerous"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/359320/pfx/drive_c/users/steamuser/Saved Games/Frontier Developments/Elite Dangerous"),
    ];

    candidates.into_iter().find(|path| path.is_dir())
}

fn apply_journal_footfall_from_directory(
    bodies: &mut [Body],
    directory: &Path,
) -> JournalFootfallReport {
    let mut report = JournalFootfallReport {
        directory: Some(directory.to_owned()),
        ..JournalFootfallReport::default()
    };
    let Some(index) = read_footfall_index(directory, &mut report.files_scanned) else {
        return report;
    };

    for body in bodies {
        let body_name = normalize_key(&body.name);
        let record = body
            .system_id64
            .as_deref()
            .map(|system_id| (format!("id:{system_id}"), body_name.clone()))
            .and_then(|key| index.get(&key))
            .or_else(|| {
                body.system_name.as_deref().and_then(|system_name| {
                    index.get(&(format!("name:{}", normalize_key(system_name)), body_name))
                })
            });

        if let Some(record) = record {
            body.journal_was_footfalled = Some(record.was_footfalled);
            body.journal_scan_timestamp = record.timestamp.clone();
            report.bodies_matched += 1;
        }
    }

    report
}

fn read_footfall_index(
    directory: &Path,
    files_scanned: &mut usize,
) -> Option<HashMap<BodyKey, FootfallRecord>> {
    let entries = fs::read_dir(directory).ok()?;
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("Journal.") && name.ends_with(".log"))
        })
        .collect::<Vec<_>>();
    paths.sort();

    let mut index = HashMap::new();
    for path in paths {
        let Ok(file) = File::open(path) else {
            continue;
        };
        *files_scanned += 1;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let Ok(line) = line else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some((keys, record)) = scan_footfall_record(&value) else {
                continue;
            };
            for key in keys {
                let replace = index
                    .get(&key)
                    .is_none_or(|current| is_newer_or_equal(&record, current));
                if replace {
                    index.insert(key, record.clone());
                }
            }
        }
    }

    Some(index)
}

fn scan_footfall_record(value: &Value) -> Option<(Vec<BodyKey>, FootfallRecord)> {
    if !value
        .get("event")
        .and_then(Value::as_str)
        .is_some_and(|event| event.eq_ignore_ascii_case("Scan"))
    {
        return None;
    }

    let was_footfalled = value.get("WasFootfalled").and_then(Value::as_bool)?;
    let body_name = string_field(value, &["BodyName", "Bodyname", "Body"])?;
    let body_name = normalize_key(&body_name);
    let mut keys = Vec::new();

    if let Some(system_address) = scalar_string(value.get("SystemAddress")) {
        keys.push((format!("id:{system_address}"), body_name.clone()));
    }
    if let Some(system_name) = string_field(value, &["StarSystem", "SystemName", "system_name"]) {
        keys.push((format!("name:{}", normalize_key(&system_name)), body_name));
    }

    if keys.is_empty() {
        return None;
    }

    Some((
        keys,
        FootfallRecord {
            was_footfalled,
            timestamp: string_field(value, &["timestamp"]),
        },
    ))
}

fn is_newer_or_equal(incoming: &FootfallRecord, current: &FootfallRecord) -> bool {
    match (incoming.timestamp.as_deref(), current.timestamp.as_deref()) {
        (Some(incoming), Some(current)) => incoming >= current,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => true,
    }
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| value.get(name).and_then(Value::as_str).map(str::to_owned))
}

fn scalar_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|value| {
        value
            .as_str()
            .map(str::to_owned)
            .or_else(|| value.as_u64().map(|id| id.to_string()))
    })
}

fn normalize_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn reads_latest_was_footfalled_value_by_system_address_and_body_name() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = env::temp_dir().join(format!(
            "ed-planet-search-journal-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("Journal.01.log"),
            concat!(
                "{\"timestamp\":\"2026-01-01T00:00:00Z\",\"event\":\"Scan\",",
                "\"StarSystem\":\"Test system\",\"SystemAddress\":123,",
                "\"BodyName\":\"Test system 1 a\",\"WasFootfalled\":false}\n",
                "{\"timestamp\":\"2026-02-01T00:00:00Z\",\"event\":\"Scan\",",
                "\"StarSystem\":\"Test system\",\"SystemAddress\":123,",
                "\"BodyName\":\"Test system 1 a\",\"WasFootfalled\":true}\n"
            ),
        )
        .unwrap();

        let mut bodies = [Body::from_json(&serde_json::json!({
            "name": "Test system 1 a",
            "system_id64": 123,
            "system_name": "Test system"
        }))
        .unwrap()];
        let report = apply_journal_footfall_from_directory(&mut bodies, &directory);

        assert_eq!(report.files_scanned, 1);
        assert_eq!(report.bodies_matched, 1);
        assert_eq!(bodies[0].journal_was_footfalled, Some(true));
        assert_eq!(
            bodies[0].journal_scan_timestamp.as_deref(),
            Some("2026-02-01T00:00:00Z")
        );

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn ignores_scans_without_was_footfalled_and_non_scan_events() {
        let missing_flag = serde_json::json!({
            "event": "Scan",
            "SystemAddress": 123,
            "BodyName": "Test 1 a"
        });
        let other_event = serde_json::json!({
            "event": "Disembark",
            "SystemAddress": 123,
            "Body": "Test 1 a",
            "WasFootfalled": true
        });

        assert!(scan_footfall_record(&missing_flag).is_none());
        assert!(scan_footfall_record(&other_event).is_none());
    }
}

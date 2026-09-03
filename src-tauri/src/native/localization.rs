use rusqlite::{params, Connection};
use serde_json::{Map, Value};

fn localized_fields(entity_type: &str) -> Option<&'static [&'static str]> {
    match entity_type {
        "captures" => Some(&["text"]),
        "problems" => Some(&["statement", "detail"]),
        "features" => Some(&["title", "outcome", "non_goals", "validation_criteria"]),
        "solution_progress_entries" => Some(&["body", "image_summary"]),
        "solution_progress_comments" => Some(&["body"]),
        "solution_checklist_items" => Some(&["body"]),
        _ => None,
    }
}

pub fn normalize_locale(locale: &str) -> &'static str {
    if locale
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
        .starts_with("ko")
    {
        "ko"
    } else {
        "en"
    }
}

fn validate_fields(entity_type: &str, fields: &Map<String, Value>) -> Result<(), String> {
    let allowed = localized_fields(entity_type).ok_or("Unsupported localized item type")?;
    if fields
        .iter()
        .any(|(key, value)| !allowed.contains(&key.as_str()) || !value.is_string())
    {
        return Err("Unregistered localized field".into());
    }
    Ok(())
}

pub fn save_versions(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
    versions: &Value,
) -> Result<(), String> {
    let versions = versions
        .as_object()
        .ok_or("localized_versions must be an object")?;
    for (locale, fields) in versions {
        let locale = normalize_locale(locale);
        let fields = fields
            .as_object()
            .ok_or("Localized fields must be an object")?;
        validate_fields(entity_type, fields)?;
        for (field, value) in fields {
            connection.execute(
                "INSERT INTO localized_content(entity_type,entity_id,field_name,locale,value,origin) VALUES (?,?,?,?,?,'ai')
                 ON CONFLICT(entity_type,entity_id,field_name,locale) DO UPDATE SET value=excluded.value,origin=excluded.origin,updated_at=CURRENT_TIMESTAMP",
                params![entity_type, entity_id, field, locale, value.as_str().unwrap_or_default()],
            ).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn supplement(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
    locale: &str,
    fields: &Value,
) -> Result<(), String> {
    let fields = fields.as_object().ok_or("fields must be an object")?;
    validate_fields(entity_type, fields)?;
    let locale = normalize_locale(locale);
    let mut statement = connection.prepare(
        "SELECT field_name,value FROM localized_content WHERE entity_type=? AND entity_id=? AND locale=?",
    ).map_err(|error| error.to_string())?;
    let mut merged = statement
        .query_map(params![entity_type, entity_id, locale], |row| {
            Ok((
                row.get::<_, String>(0)?,
                Value::String(row.get::<_, String>(1)?),
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Map<_, _>, _>>()
        .map_err(|error| error.to_string())?;
    merged.extend(fields.clone());
    let mut versions = Map::new();
    versions.insert(locale.to_owned(), Value::Object(merged));
    save_versions(connection, entity_type, entity_id, &Value::Object(versions))
}

pub fn overlay(
    connection: &Connection,
    entity_type: &str,
    mut record: Value,
    requested_locale: &str,
) -> Result<Value, String> {
    let locale = normalize_locale(requested_locale);
    let mut statement = connection
        .prepare(
            "SELECT locale,field_name,value FROM localized_content WHERE entity_type=? AND entity_id=? ORDER BY locale DESC,field_name",
        )
        .map_err(|error| error.to_string())?;
    let entity_id = record
        .get("id")
        .and_then(Value::as_str)
        .ok_or("Localized record id is missing")?;
    let versions = statement
        .query_map(params![entity_type, entity_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .into_iter()
        .fold(Map::new(), |mut versions, (locale, field, value)| {
            versions
                .entry(locale)
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
                .expect("localized version is an object")
                .insert(field, Value::String(value));
            versions
        });
    let selected = versions.get(locale).and_then(Value::as_object);
    let allowed = localized_fields(entity_type).ok_or("Unsupported localized item type")?;
    let mut fallback = selected.is_none();
    if let (Some(record), Some(selected)) = (record.as_object_mut(), selected) {
        for field in allowed {
            if let Some(value) = selected.get(*field) {
                record.insert((*field).to_owned(), value.clone());
            } else {
                fallback = true;
            }
        }
    }
    let record = record
        .as_object_mut()
        .ok_or("Localized record must be an object")?;
    record.insert(
        "content_locale".into(),
        Value::String(
            if selected.is_some() {
                locale
            } else {
                "original"
            }
            .into(),
        ),
    );
    record.insert(
        "available_locales".into(),
        Value::Array(
            ["ko", "en"]
                .into_iter()
                .filter(|candidate| versions.contains_key(*candidate))
                .map(|candidate| Value::String(candidate.into()))
                .collect(),
        ),
    );
    record.insert("fallback_used".into(), Value::Bool(fallback));
    record.insert("localized_versions".into(), Value::Object(versions));
    Ok(Value::Object(record.clone()))
}

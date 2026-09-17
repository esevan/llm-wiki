use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::{params, Connection};
#[cfg(test)]
use rusqlite::OptionalExtension;
use serde_json::{json, Value};

pub(crate) fn validate(input: &Value) -> Result<Option<Value>, String> {
    let Some(image) = input.get("image").filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let media = image["mediaType"].as_str().unwrap_or("");
    let data = image["data"].as_str().unwrap_or("");
    if data.len() > 14_000_000 {
        return Err("invalid_input: image exceeds 10 MB".into());
    }
    let bytes = STANDARD
        .decode(data)
        .map_err(|_| "invalid_input: invalid image encoding")?;
    let valid = match media {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
        _ => false,
    };
    if !valid || bytes.len() > 10 * 1024 * 1024 {
        return Err("invalid_input: use a PNG, JPEG, GIF or WebP image up to 10 MB".into());
    }
    Ok(Some(
        json!({"name": image["name"].as_str().unwrap_or("image"), "mediaType":media, "data":data}),
    ))
}

pub(crate) fn save(
    connection: &Connection,
    capture: bool,
    id: &str,
    image: &Value,
) -> Result<(), String> {
    connection.execute("INSERT INTO input_images(capture_id,message_id,name,media_type,data) VALUES(?,?,?,?,?)",
        params![if capture { Some(id) } else { None }, if capture { None } else { Some(id) },
            image["name"].as_str(), image["mediaType"].as_str(), image["data"].as_str()])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn get(
    connection: &Connection,
    capture: bool,
    id: &str,
) -> Result<Option<Value>, String> {
    connection.query_row(if capture {
        "SELECT name,media_type,data FROM input_images WHERE capture_id=?"
    } else {
        "SELECT name,media_type,data FROM input_images WHERE message_id=?"
    }, [id], |row| Ok(json!({"name":row.get::<_,String>(0)?,"mediaType":row.get::<_,String>(1)?,"data":row.get::<_,String>(2)?})))
        .optional().map_err(|error| error.to_string())
}

/// The array form takes precedence; legacy single-image requests remain valid.
pub(crate) fn validate_all(input: &Value) -> Result<Vec<Value>, String> {
    if let Some(images) = input.get("images").filter(|value| !value.is_null()) {
        let images = images.as_array().ok_or("invalid_input: images must be an array")?;
        images.iter().map(|image| validate(&json!({"image": image}))?
            .ok_or_else(|| "invalid_input: image must be an object".into())).collect()
    } else {
        Ok(validate(input)?.into_iter().collect())
    }
}

pub(crate) fn get_all(connection: &Connection, capture: bool, id: &str) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(if capture {
        "SELECT name,media_type,data FROM input_images WHERE capture_id=? ORDER BY rowid"
    } else {
        "SELECT name,media_type,data FROM input_images WHERE message_id=? ORDER BY rowid"
    }).map_err(|error| error.to_string())?;
    let rows = statement.query_map([id], |row| Ok(json!({"name":row.get::<_,String>(0)?,"mediaType":row.get::<_,String>(1)?,"data":row.get::<_,String>(2)?})))
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
}

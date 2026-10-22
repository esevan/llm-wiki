/// Titles use Unicode scalar values, matching JSON Schema maxLength and JS Array.from.
pub const MAX_TITLE_CHARACTERS: usize = 70;
pub fn validate_title(title: &str) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("invalid_input: title is required".into());
    }
    if title.chars().count() > MAX_TITLE_CHARACTERS {
        return Err("invalid_input: title must be at most 70 Unicode characters".into());
    }
    Ok(())
}

use sha2::{Digest, Sha256};

pub const READINESS_FIELDS: [&str; 4] = ["outcome", "scope", "validationCriteria", "prerequisites"];

pub fn content_hash(fields: &[&str]) -> String {
    format!("{:x}", Sha256::digest(fields.join("\u{1f}").as_bytes()))
}

/// Only an explicit lifecycle reopen can permit further content changes.
pub fn ensure_content_mutable(state: &str) -> Result<(), String> {
    if state == "completed" {
        return Err("task_completed: completed Task content and history are read-only".into());
    }
    Ok(())
}

pub fn validate_state_transition(from: &str, to: &str) -> Result<String, String> {
    match (from, to) {
        ("task", "in_progress") | ("in_progress", "completed") | ("in_progress", "on_hold") => Ok(to.into()),
        ("on_hold", "resume") => Ok("in_progress".into()),
        ("in_progress" | "on_hold", "backlog") => Ok("task".into()),
        ("completed", "reopen") => Ok("in_progress".into()),
        _ => Err(format!("transition_invalid: {from} to {to}")),
    }
}

pub fn validate_relationship(source: &str, target: &str, kind: &str) -> Result<(), String> {
    if source == target {
        return Err("relationship_invalid: self links are not allowed".into());
    }
    if !["prerequisite", "split_from", "related"].contains(&kind) {
        return Err("relationship_invalid: unsupported relationship".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_requires_explicit_reopen() {
        assert_eq!(
            validate_state_transition("task", "in_progress").unwrap(),
            "in_progress"
        );
        assert!(validate_state_transition("completed", "task").is_err());
        assert_eq!(
            validate_state_transition("completed", "reopen").unwrap(),
            "in_progress"
        );
    }
    #[test]
    fn relationship_rejects_self() {
        assert!(validate_relationship("a", "a", "related").is_err());
    }
}

#[cfg(test)]
mod title_tests {
    use super::*;
    #[test]
    fn titles_count_unicode_scalars_not_bytes_or_utf16_units() {
        for character in ["가", "😀", "a"] {
            assert!(validate_title(&character.repeat(70)).is_ok());
            assert!(validate_title(&character.repeat(71)).is_err());
        }
        assert!(validate_title("  ").is_err());
        // Combining marks and ZWJ sequences count as their Unicode scalar values.
        assert_eq!("e\u{301}".chars().count(), 2);
        assert_eq!("👨‍👩‍👧‍👦".chars().count(), 7);
        assert!(validate_title(&"👨‍👩‍👧‍👦".repeat(10)).is_ok());
        assert!(validate_title(&("👨‍👩‍👧‍👦".repeat(10) + "x")).is_err());
    }
}

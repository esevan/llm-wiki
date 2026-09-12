use sha2::{Digest, Sha256};

pub const READINESS_FIELDS: [&str; 4] = ["outcome", "scope", "validationCriteria", "prerequisites"];

pub fn content_hash(fields: &[&str]) -> String {
    format!("{:x}", Sha256::digest(fields.join("\u{1f}").as_bytes()))
}

pub fn validate_state_transition(from: &str, to: &str) -> Result<String, String> {
    match (from, to) {
        ("task", "in_progress") | ("in_progress", "completed") => Ok(to.into()),
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

use std::collections::BTreeSet;

use axum::http::StatusCode;
use lifetrace_contracts::auth::v1::Scope;
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::ErrorCode;

use crate::error::ApiError;

pub const ALL_SCOPES: &[&str] = &[
    "account:read",
    "account:write",
    "devices:read",
    "devices:write",
    "sessions:read",
    "sessions:write",
    "sync:read",
    "sync:write",
    "finance:read",
    "finance:write",
    "notes:read",
    "notes:write",
    "files:read",
    "files:write",
    "english:read",
    "english:write",
    "habits:read",
    "habits:write",
    "reviews:read",
    "reviews:write",
    "workouts:read",
    "workouts:write",
];

pub fn supported_app(app_id: &str) -> bool {
    matches!(
        app_id,
        AppId::DESKTOP
            | AppId::FINANCE_ANDROID
            | AppId::NOTES_ANDROID
            | AppId::ENGLISH_ANDROID
            | AppId::HABITS_ANDROID
            | AppId::WEB
    )
}

pub fn allowed_scopes(app_id: &str) -> BTreeSet<String> {
    let values: &[&str] = match app_id {
        AppId::DESKTOP => ALL_SCOPES,
        AppId::FINANCE_ANDROID => &[
            "account:read",
            "devices:read",
            "sync:read",
            "sync:write",
            "finance:read",
            "finance:write",
        ],
        AppId::NOTES_ANDROID => &[
            "account:read",
            "devices:read",
            "sync:read",
            "sync:write",
            "notes:read",
            "notes:write",
            "files:read",
            "files:write",
        ],
        AppId::ENGLISH_ANDROID => &[
            "account:read",
            "devices:read",
            "sync:read",
            "sync:write",
            "english:read",
            "english:write",
        ],
        AppId::HABITS_ANDROID => &[
            "account:read",
            "devices:read",
            "sync:read",
            "sync:write",
            "habits:read",
            "habits:write",
            "reviews:read",
            "reviews:write",
        ],
        AppId::WEB => ALL_SCOPES,
        _ => &[],
    };
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub fn default_scopes(app_id: &str) -> Vec<String> {
    allowed_scopes(app_id).into_iter().collect()
}

pub fn issue_scopes(app_id: &str, requested: &[Scope], granted: &[String]) -> Vec<String> {
    let allowed = allowed_scopes(app_id);
    let grant: BTreeSet<&str> = granted.iter().map(String::as_str).collect();
    let requested_values: BTreeSet<&str> = if requested.is_empty() {
        grant.clone()
    } else {
        requested.iter().map(Scope::as_str).collect()
    };
    requested_values
        .into_iter()
        .filter(|value| allowed.contains(*value) && grant.contains(*value))
        .map(str::to_owned)
        .collect()
}

pub fn required_entity_scope(entity_type: &str, write: bool) -> Option<&'static str> {
    let suffix = if write { "write" } else { "read" };
    let domain = if entity_type == "identity.user" {
        "account"
    } else if entity_type == "identity.device" {
        "devices"
    } else if entity_type.starts_with("finance.") {
        "finance"
    } else if entity_type.starts_with("note.") {
        "notes"
    } else if entity_type.starts_with("english.") {
        "english"
    } else if entity_type.starts_with("habit.") {
        "habits"
    } else if entity_type == "review.daily" {
        "reviews"
    } else if entity_type.starts_with("workout.") || entity_type == "training.note" {
        "workouts"
    } else if entity_type == "file.metadata" {
        "files"
    } else if entity_type == "user.preference" || entity_type == "entity.link" {
        "account"
    } else {
        return None;
    };
    Some(match (domain, suffix) {
        ("finance", "read") => "finance:read",
        ("finance", _) => "finance:write",
        ("notes", "read") => "notes:read",
        ("notes", _) => "notes:write",
        ("english", "read") => "english:read",
        ("english", _) => "english:write",
        ("habits", "read") => "habits:read",
        ("habits", _) => "habits:write",
        ("reviews", "read") => "reviews:read",
        ("reviews", _) => "reviews:write",
        ("workouts", "read") => "workouts:read",
        ("workouts", _) => "workouts:write",
        ("files", "read") => "files:read",
        ("files", _) => "files:write",
        ("account", "read") => "account:read",
        ("account", _) => "account:write",
        ("devices", "read") => "devices:read",
        ("devices", _) => "devices:write",
        _ => return None,
    })
}

pub fn require(scopes: &BTreeSet<String>, required: &str) -> Result<(), ApiError> {
    if scopes.contains(required) {
        Ok(())
    } else {
        Err(ApiError::new(
            ErrorCode::AuthScopeDenied,
            format!("required scope is not granted: {required}"),
            StatusCode::FORBIDDEN,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finance_app_cannot_receive_notes_scope() {
        let granted = default_scopes(AppId::FINANCE_ANDROID);
        let issued = issue_scopes(
            AppId::FINANCE_ANDROID,
            &[Scope::new("notes:read"), Scope::new("finance:read")],
            &granted,
        );
        assert_eq!(issued, vec!["finance:read"]);
    }

    #[test]
    fn unknown_entity_type_never_defaults_to_allow() {
        assert_eq!(required_entity_scope("future.secret", false), None);
    }
}

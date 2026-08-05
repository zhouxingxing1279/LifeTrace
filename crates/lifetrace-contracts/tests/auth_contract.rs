use lifetrace_contracts::auth::v1::{
    AuthCapabilitiesV1, LoginRequestV1, RefreshRequestV1, Scope, TokenResponseV1,
};
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::ErrorCode;

#[test]
fn auth_error_codes_have_stable_wire_names() {
    assert_eq!(
        ErrorCode::AuthRefreshTokenReused.wire_name(),
        "LIFETRACE_AUTH_REFRESH_TOKEN_REUSED"
    );
    assert_eq!(
        ErrorCode::AuthCsrfInvalid.wire_name(),
        "LIFETRACE_AUTH_CSRF_INVALID"
    );
    assert_eq!(
        ErrorCode::AuthRegistrationDisabled.wire_name(),
        "LIFETRACE_AUTH_REGISTRATION_DISABLED"
    );
}

#[test]
fn login_contract_uses_camel_case_and_never_contains_hash_fields() {
    let request = LoginRequestV1 {
        email: "user@example.com".to_owned(),
        password: "a long password phrase".to_owned(),
        app_id: AppId::new(AppId::DESKTOP),
        device_id: "device".to_owned(),
        device_name: "Desktop".to_owned(),
        platform: "windows".to_owned(),
        client_version: Some("0.2.1".to_owned()),
        requested_scopes: vec![Scope::new("sync:read")],
        public_device: false,
    };
    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["appId"], AppId::DESKTOP);
    assert!(value.get("passwordHash").is_none());
    assert!(value.get("tokenHash").is_none());
}

#[test]
fn public_auth_contracts_generate_json_schema() {
    let login = schemars::schema_for!(LoginRequestV1);
    let refresh = schemars::schema_for!(RefreshRequestV1);
    let response = schemars::schema_for!(TokenResponseV1);
    let capabilities = schemars::schema_for!(AuthCapabilitiesV1);
    for schema in [login, refresh, response, capabilities] {
        assert!(!serde_json::to_value(schema).unwrap().is_null());
    }
}

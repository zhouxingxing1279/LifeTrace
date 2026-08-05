# EPIC-04 Completion Report

## Status

Completed and validated before publication to `agent/epic-04-auth-complete`.

## Delivered scope

- Argon2id password hashing with configurable pepper and production fail-closed checks.
- Controlled registration, bootstrap-user and invite administration flows.
- Opaque access tokens and rotating refresh-token families with replay detection and family revocation.
- PostgreSQL persistence for users, application grants, devices, sessions, tokens, password resets, rate limits and audit events.
- Asynchronous database authentication provider and scoped principals.
- Native login, refresh, logout, logout-all, device/session management and password lifecycle APIs.
- Secure web sessions with HttpOnly cookies, CSRF validation and trusted-proxy-aware request context.
- Windows Credential Manager integration for refresh-token storage; access tokens remain memory-only.
- Local desktop mode remains login-free and continues to use local SQLite.
- Generated Rust, JSON Schema, OpenAPI and TypeScript authentication contracts.
- Scope-aware sync authorization while preserving EPIC-03 cursor and batch-rejection compatibility.

## Validation gates

The publishing workflow requires all of the following to pass before the implementation commit is pushed:

- Contract generation and contract tests.
- PostgreSQL authentication and API integration tests.
- Existing sync protocol/API regression tests.
- Desktop Rust tests.
- Frontend tests and production build.
- Rust Clippy with warnings denied.
- Windows Tauri and Credential Manager compilation.
- Cloud Docker image build.

## Security notes

- Passwords and raw tokens are never persisted.
- Refresh-token reuse revokes the entire token family.
- Application grants and entity scopes are enforced server-side.
- Unknown entity types retain stable per-item rejection semantics and are never implicitly authorized.
- Production configuration rejects development credentials, weak/default peppers, insecure cookies and non-HTTPS public URLs.

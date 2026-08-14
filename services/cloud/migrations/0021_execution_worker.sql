-- P4: multi-instance-safe lease for the dedicated Execution maintenance worker.
-- The worker owns no user data here; user-facing reminders/occurrences remain
-- normal sync entities so every client observes the same authoritative state.

CREATE TABLE execution_worker_leases (
    lease_name TEXT PRIMARY KEY,
    owner_id UUID NOT NULL,
    lease_until TIMESTAMPTZ NOT NULL,
    heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_execution_worker_leases_expiry
    ON execution_worker_leases(lease_until);

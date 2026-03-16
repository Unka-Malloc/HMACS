-- Carbide Compliance Schema

-- KYC Records
CREATE TABLE IF NOT EXISTS kyc_records (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    jurisdiction VARCHAR(8) NOT NULL,
    tier VARCHAR(16) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'not_started',
    legal_name_encrypted TEXT,
    nationality VARCHAR(4),
    country_of_residence VARCHAR(4),
    date_of_birth_encrypted TEXT,
    risk_score INTEGER,
    reviewer_notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ
);

CREATE INDEX idx_kyc_participant ON kyc_records(participant_id);
CREATE INDEX idx_kyc_status ON kyc_records(status);
CREATE INDEX idx_kyc_jurisdiction ON kyc_records(jurisdiction);

-- KYC Documents
CREATE TABLE IF NOT EXISTS kyc_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kyc_record_id UUID NOT NULL REFERENCES kyc_records(id),
    document_type VARCHAR(64) NOT NULL,
    content_hash VARCHAR(128) NOT NULL,
    storage_ref TEXT NOT NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    verified_at TIMESTAMPTZ,
    rejection_reason TEXT
);

CREATE INDEX idx_kyc_docs_record ON kyc_documents(kyc_record_id);

-- Sanctions / AML Screening Records
CREATE TABLE IF NOT EXISTS screening_records (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    screened_name TEXT,
    screened_address VARCHAR(255),
    result VARCHAR(32) NOT NULL,
    lists_checked JSONB NOT NULL DEFAULT '[]',
    screened_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ,
    reviewer_id UUID REFERENCES participants(id),
    reviewer_notes TEXT
);

CREATE INDEX idx_screening_participant ON screening_records(participant_id);
CREATE INDEX idx_screening_result ON screening_records(result);

-- Screening Matches
CREATE TABLE IF NOT EXISTS screening_matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screening_id UUID NOT NULL REFERENCES screening_records(id),
    sanctions_list VARCHAR(64) NOT NULL,
    category VARCHAR(32) NOT NULL,
    matched_name TEXT NOT NULL,
    match_score DOUBLE PRECISION NOT NULL,
    list_entry_id VARCHAR(255),
    details TEXT
);

CREATE INDEX idx_matches_screening ON screening_matches(screening_id);

-- Compliance Alerts
CREATE TABLE IF NOT EXISTS compliance_alerts (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    alert_type VARCHAR(64) NOT NULL,
    severity VARCHAR(16) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'open',
    jurisdiction VARCHAR(8) NOT NULL,
    transaction_id UUID,
    amount DECIMAL(38, 18),
    asset VARCHAR(16),
    description TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_by VARCHAR(255),
    resolution_notes TEXT
);

CREATE INDEX idx_alerts_participant ON compliance_alerts(participant_id);
CREATE INDEX idx_alerts_status ON compliance_alerts(status);
CREATE INDEX idx_alerts_type ON compliance_alerts(alert_type);
CREATE INDEX idx_alerts_severity ON compliance_alerts(severity);

-- Travel Rule Messages
CREATE TABLE IF NOT EXISTS travel_rule_messages (
    id UUID PRIMARY KEY,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    jurisdiction VARCHAR(8) NOT NULL,
    amount DECIMAL(38, 18) NOT NULL,
    asset VARCHAR(16) NOT NULL,
    originator_participant_id UUID NOT NULL REFERENCES participants(id),
    originator_name TEXT,
    originator_address VARCHAR(255),
    originator_account_ref VARCHAR(255),
    beneficiary_participant_id UUID NOT NULL REFERENCES participants(id),
    beneficiary_name TEXT,
    beneficiary_address VARCHAR(255),
    beneficiary_account_ref VARCHAR(255),
    originating_vasp_name VARCHAR(255) NOT NULL,
    originating_vasp_lei VARCHAR(32),
    originating_vasp_jurisdiction VARCHAR(8),
    beneficiary_vasp_name VARCHAR(255),
    beneficiary_vasp_lei VARCHAR(32),
    tx_hash VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ
);

CREATE INDEX idx_travel_rule_status ON travel_rule_messages(status);
CREATE INDEX idx_travel_rule_originator ON travel_rule_messages(originator_participant_id);
CREATE INDEX idx_travel_rule_beneficiary ON travel_rule_messages(beneficiary_participant_id);

-- Risk Assessments
CREATE TABLE IF NOT EXISTS risk_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    participant_id UUID NOT NULL REFERENCES participants(id),
    overall_score INTEGER NOT NULL,
    max_possible_score INTEGER NOT NULL,
    risk_level VARCHAR(16) NOT NULL,
    factors JSONB NOT NULL DEFAULT '[]',
    assessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    next_review_at TIMESTAMPTZ
);

CREATE INDEX idx_risk_participant ON risk_assessments(participant_id);
CREATE INDEX idx_risk_level ON risk_assessments(risk_level);

-- GDPR Consent Records
CREATE TABLE IF NOT EXISTS consent_records (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    data_category VARCHAR(32) NOT NULL,
    legal_basis VARCHAR(32) NOT NULL,
    purpose TEXT NOT NULL,
    granted BOOLEAN NOT NULL DEFAULT FALSE,
    granted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    source VARCHAR(255),
    jurisdiction VARCHAR(8) NOT NULL
);

CREATE INDEX idx_consent_participant ON consent_records(participant_id);
CREATE INDEX idx_consent_active ON consent_records(granted, revoked_at);

-- Data Erasure Requests
CREATE TABLE IF NOT EXISTS erasure_requests (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    status VARCHAR(32) NOT NULL DEFAULT 'requested',
    categories_requested JSONB NOT NULL DEFAULT '[]',
    categories_completed JSONB NOT NULL DEFAULT '[]',
    categories_retained JSONB NOT NULL DEFAULT '[]',
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    processor_notes TEXT
);

CREATE INDEX idx_erasure_participant ON erasure_requests(participant_id);
CREATE INDEX idx_erasure_status ON erasure_requests(status);

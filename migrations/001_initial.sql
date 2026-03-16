-- HMACS Initial Schema

-- Participants (humans and agents)
CREATE TABLE IF NOT EXISTS participants (
    id UUID PRIMARY KEY,
    kind VARCHAR(16) NOT NULL CHECK (kind IN ('human', 'agent')),
    display_name VARCHAR(255),
    wallet_address VARCHAR(255),
    wallet_chain VARCHAR(32),
    capabilities JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_participants_kind ON participants(kind);
CREATE INDEX idx_participants_wallet ON participants(wallet_address);

-- API Keys
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    key_hash VARCHAR(128) NOT NULL UNIQUE,
    label VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    revoked BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_api_keys_participant ON api_keys(participant_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);

-- Tasks
CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY,
    creator_id UUID NOT NULL REFERENCES participants(id),
    title VARCHAR(512) NOT NULL,
    description TEXT NOT NULL,
    skill_tags JSONB NOT NULL DEFAULT '[]',
    budget_asset VARCHAR(16) NOT NULL,
    budget_amount DECIMAL(38, 18) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'open',
    restriction VARCHAR(32) NOT NULL DEFAULT 'any',
    assigned_to UUID REFERENCES participants(id),
    deadline TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    deliverable_url TEXT,
    deliverable_notes TEXT
);

CREATE INDEX idx_tasks_creator ON tasks(creator_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_assigned ON tasks(assigned_to);

-- Bids
CREATE TABLE IF NOT EXISTS bids (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES tasks(id),
    bidder_id UUID NOT NULL REFERENCES participants(id),
    amount DECIMAL(38, 18) NOT NULL,
    asset VARCHAR(16) NOT NULL,
    proposal TEXT NOT NULL,
    estimated_duration_hours DOUBLE PRECISION,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bids_task ON bids(task_id);
CREATE INDEX idx_bids_bidder ON bids(bidder_id);

-- Compute Resources
CREATE TABLE IF NOT EXISTS compute_resources (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES participants(id),
    cpu_cores INTEGER NOT NULL,
    memory_gb INTEGER NOT NULL,
    gpu JSONB,
    bandwidth_mbps INTEGER,
    storage_gb INTEGER,
    status VARCHAR(32) NOT NULL DEFAULT 'available',
    available_from TIMESTAMPTZ,
    available_until TIMESTAMPTZ,
    region VARCHAR(64),
    tags JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_compute_resources_owner ON compute_resources(owner_id);
CREATE INDEX idx_compute_resources_status ON compute_resources(status);

-- Compute Listings
CREATE TABLE IF NOT EXISTS compute_listings (
    id UUID PRIMARY KEY,
    resource_id UUID NOT NULL REFERENCES compute_resources(id),
    seller_id UUID NOT NULL REFERENCES participants(id),
    pricing JSONB NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_compute_listings_seller ON compute_listings(seller_id);
CREATE INDEX idx_compute_listings_active ON compute_listings(active);

-- Compute Leases
CREATE TABLE IF NOT EXISTS compute_leases (
    id UUID PRIMARY KEY,
    resource_id UUID NOT NULL REFERENCES compute_resources(id),
    listing_id UUID NOT NULL REFERENCES compute_listings(id),
    buyer_id UUID NOT NULL REFERENCES participants(id),
    seller_id UUID NOT NULL REFERENCES participants(id),
    asset VARCHAR(16) NOT NULL,
    price_per_unit DECIMAL(38, 18) NOT NULL,
    unit_label VARCHAR(64) NOT NULL,
    units_consumed DECIMAL(38, 18) NOT NULL DEFAULT 0,
    max_units DECIMAL(38, 18),
    total_cost DECIMAL(38, 18) NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_metered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ
);

CREATE INDEX idx_compute_leases_buyer ON compute_leases(buyer_id);
CREATE INDEX idx_compute_leases_seller ON compute_leases(seller_id);
CREATE INDEX idx_compute_leases_status ON compute_leases(status);

-- Wallets / Balances
CREATE TABLE IF NOT EXISTS balances (
    wallet_id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    asset VARCHAR(16) NOT NULL,
    available DECIMAL(38, 18) NOT NULL DEFAULT 0,
    frozen DECIMAL(38, 18) NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(participant_id, asset)
);

CREATE INDEX idx_balances_participant ON balances(participant_id);

-- Ledger Entries (immutable transaction log)
CREATE TABLE IF NOT EXISTS ledger_entries (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    entry_type VARCHAR(32) NOT NULL,
    asset VARCHAR(16) NOT NULL,
    amount DECIMAL(38, 18) NOT NULL,
    direction SMALLINT NOT NULL,
    balance_after DECIMAL(38, 18) NOT NULL,
    reference_id VARCHAR(255),
    reference_type VARCHAR(64),
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ledger_participant ON ledger_entries(participant_id);
CREATE INDEX idx_ledger_type ON ledger_entries(entry_type);
CREATE INDEX idx_ledger_created ON ledger_entries(created_at);

-- Settlement Records
CREATE TABLE IF NOT EXISTS settlement_deposits (
    id UUID PRIMARY KEY,
    chain VARCHAR(32) NOT NULL,
    tx_hash VARCHAR(255) NOT NULL UNIQUE,
    from_address VARCHAR(255) NOT NULL,
    to_address VARCHAR(255) NOT NULL,
    token_address VARCHAR(255),
    amount DECIMAL(38, 18) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    block_number BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ
);

CREATE INDEX idx_settlement_deposits_status ON settlement_deposits(status);
CREATE INDEX idx_settlement_deposits_tx ON settlement_deposits(tx_hash);

CREATE TABLE IF NOT EXISTS settlement_withdrawals (
    id UUID PRIMARY KEY,
    chain VARCHAR(32) NOT NULL,
    to_address VARCHAR(255) NOT NULL,
    token_address VARCHAR(255),
    amount DECIMAL(38, 18) NOT NULL,
    decimals SMALLINT NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    tx_hash VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    error_message TEXT
);

CREATE INDEX idx_settlement_withdrawals_status ON settlement_withdrawals(status);

-- Orders (for matching engine persistence)
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL REFERENCES participants(id),
    side VARCHAR(8) NOT NULL CHECK (side IN ('buy', 'sell')),
    order_type VARCHAR(16) NOT NULL CHECK (order_type IN ('market', 'limit')),
    asset VARCHAR(16) NOT NULL,
    price DECIMAL(38, 18) NOT NULL,
    quantity DECIMAL(38, 18) NOT NULL,
    filled_quantity DECIMAL(38, 18) NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL DEFAULT 'open',
    reference_id VARCHAR(255),
    reference_type VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

CREATE INDEX idx_orders_participant ON orders(participant_id);
CREATE INDEX idx_orders_status ON orders(status);
CREATE INDEX idx_orders_asset_side ON orders(asset, side);

-- Trade Executions
CREATE TABLE IF NOT EXISTS trade_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    buy_order_id UUID NOT NULL REFERENCES orders(id),
    sell_order_id UUID NOT NULL REFERENCES orders(id),
    price DECIMAL(38, 18) NOT NULL,
    quantity DECIMAL(38, 18) NOT NULL,
    asset VARCHAR(16) NOT NULL,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_trades_buy ON trade_executions(buy_order_id);
CREATE INDEX idx_trades_sell ON trade_executions(sell_order_id);
CREATE INDEX idx_trades_executed ON trade_executions(executed_at);

-- Metering Events
CREATE TABLE IF NOT EXISTS metering_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    lease_id UUID NOT NULL REFERENCES compute_leases(id),
    units DECIMAL(38, 18) NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_metering_lease ON metering_events(lease_id);

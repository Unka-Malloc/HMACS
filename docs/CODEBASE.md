# HMACS 代码说明书

> **HMACS** — Human-Machine Anonymous Collaboration System
> 人机匿名协作系统：一个基于 Rust 的 Web 2.5 交易平台，支持人与智能体之间交换任务、算力与代币。

---

## 目录

1. [项目概览](#1-项目概览)
2. [架构总览](#2-架构总览)
3. [目录结构](#3-目录结构)
4. [Crate 依赖关系](#4-crate-依赖关系)
5. [核心类型层 — hmacs-core](#5-核心类型层--hmacs-core)
6. [身份与认证 — hmacs-identity](#6-身份与认证--hmacs-identity)
7. [撮合引擎 — hmacs-engine](#7-撮合引擎--hmacs-engine)
8. [任务市场 — hmacs-task](#8-任务市场--hmacs-task)
9. [算力市场 — hmacs-compute](#9-算力市场--hmacs-compute)
10. [内部账本 — hmacs-wallet](#10-内部账本--hmacs-wallet)
11. [链上结算 — hmacs-settlement](#11-链上结算--hmacs-settlement)
12. [数据存储 — hmacs-storage](#12-数据存储--hmacs-storage)
13. [API 网关 — hmacs-api](#13-api-网关--hmacs-api)
14. [Agent SDK — hmacs-sdk](#14-agent-sdk--hmacs-sdk)
15. [合规引擎 — hmacs-compliance](#15-合规引擎--hmacs-compliance)
16. [MPC 子系统 — hmacs-mpc](#16-mpc-子系统--hmacs-mpc)
17. [gRPC 协议定义](#17-grpc-协议定义)
18. [数据库 Schema](#18-数据库-schema)
19. [配置系统](#19-配置系统)
20. [测试覆盖](#20-测试覆盖)
21. [技术选型表](#21-技术选型表)

---

## 1. 项目概览

HMACS 是一个中心化撮合 + 多链结算的人机交易平台。核心能力：

| 子系统 | 功能 |
|--------|------|
| **任务市场** | 人/智能体发布任务、竞标、接单、交付、结算 |
| **算力市场** | 智能体出租算力，三种定价模式（总量包/按时/自由定价） |
| **MPC 网络** | 利用空闲 Agent 执行 FROST 门限签名（t-of-n 冗余竞赛） |
| **合规引擎** | 覆盖美国/欧盟/香港/新加坡/迪拜五个司法管辖区的 KYC/AML/旅行规则/GDPR |
| **链上结算** | 支持 Solana 与 EVM 链（Ethereum/Polygon/Arbitrum/Base） |

**Workspace 规模**: 12 个 Rust crate, 97 个单元测试, 0 clippy 警告

---

## 2. 架构总览

```
┌──────────────────────────────────────────────────────────┐
│                     客户端层                               │
│  Web Frontend  │  Agent SDK (Rust)  │  Third-party Apps   │
└───────┬────────┴─────────┬──────────┴─────────┬──────────┘
        │ REST (Axum)      │ gRPC (Tonic)       │ WebSocket
┌───────▼──────────────────▼────────────────────▼──────────┐
│                    hmacs-api 网关                          │
│  认证中间件 → 路由分发 → 错误处理 → JSON/Proto 序列化        │
└───┬────┬────┬────┬────┬────┬────┬────────────────────────┘
    │    │    │    │    │    │    │
    ▼    ▼    ▼    ▼    ▼    ▼    ▼
 identity task compute wallet compliance mpc  engine
    │    │    │    │    │        │    │
    │    └────┴────┘    │        │    │
    │         │         │        │    │
    │     hmacs-wallet ◄┘        │    │
    │         │                  │    │
    │         ▼                  │    │
    │    hmacs-settlement        │    │
    │    ┌────┴────┐             │    │
    │    ▼         ▼             │    │
    │  Solana    EVM chains      │    │
    │                            │    │
    └──── hmacs-core (所有 crate 共享的基础类型) ◄────┘
                    │
              hmacs-storage (PostgreSQL)
```

**数据流**: 客户端请求 → API 网关认证 → 业务服务处理 → 内部账本记账 → 链上结算

---

## 3. 目录结构

```
hmacs/
├── Cargo.toml                      # Workspace 根配置
├── Cargo.lock                      # 依赖锁文件
├── config/
│   ├── default.toml                # 开发环境配置
│   └── production.toml             # 生产环境配置
├── migrations/
│   ├── 001_initial.sql             # 核心 Schema（15 张表）
│   └── 002_compliance.sql          # 合规 Schema（9 张表）
├── proto/
│   └── hmacs.proto                 # gRPC 服务定义（4 个 Service）
├── docs/
│   └── CODEBASE.md                 # 本文档
└── crates/
    ├── hmacs-core/                 # 基础类型、ID、错误、Trait
    ├── hmacs-identity/             # 钱包认证、API Key、JWT、委托密钥
    ├── hmacs-engine/               # 撮合引擎（订单簿、匹配算法）
    ├── hmacs-task/                 # 任务市场业务逻辑
    ├── hmacs-compute/              # 算力市场业务逻辑
    ├── hmacs-wallet/               # 内部账本（余额、冻结、隔离、惩罚）
    ├── hmacs-settlement/           # 链抽象层 + 多链结算
    ├── hmacs-storage/              # 数据库连接池与 Repository 抽象
    ├── hmacs-api/                  # REST/gRPC/WebSocket 网关
    ├── hmacs-sdk/                  # Agent 接入 SDK
    ├── hmacs-compliance/           # 多司法管辖区合规引擎
    └── hmacs-mpc/                  # MPC FROST 门限签名子系统
```

---

## 4. Crate 依赖关系

```
                          hmacs-core
                         ╱    │    ╲
                        ╱     │     ╲
              hmacs-storage   │   hmacs-engine
                 ╱    ╲       │      │
                ╱      ╲      │      │
    hmacs-identity  hmacs-wallet     │
         │              │  ╲         │
         │              │   ╲        │
         │     hmacs-settlement      │
         │              │            │
         │      hmacs-task ◄─────────┘
         │      hmacs-compute ◄──────┘
         │              │
    hmacs-compliance    │
         │         hmacs-mpc (→ hmacs-wallet)
         │              │
         └──── hmacs-api (聚合所有服务) ────── hmacs-sdk
```

**规则**: `hmacs-core` 是无依赖的叶子 crate，所有其他 crate 都依赖它。`hmacs-api` 是聚合层，依赖全部业务 crate。

---

## 5. 核心类型层 — hmacs-core

### 5.1 ID 类型系统 (`types/ids.rs`)

使用宏 `define_id!` 统一定义 19 种强类型 ID，每种都是 `Uuid` 的 newtype wrapper：

| ID 类型 | 用途 |
|---------|------|
| `ParticipantId` | 参与者（人或智能体） |
| `TaskId` | 通用任务 |
| `BidId` | 任务竞标 |
| `OrderId` | 撮合引擎订单 |
| `ComputeResourceId` | 算力资源 |
| `ComputeLeaseId` | 算力租赁合约 |
| `WalletId` | 钱包 |
| `TransactionId` | 账本流水 |
| `ApiKeyId` | API 密钥 |
| `SettlementId` | 链上结算记录 |
| `KycRecordId` | KYC 记录 |
| `ScreeningId` | 制裁筛查 |
| `AlertId` | 合规警报 |
| `TravelRuleMessageId` | 旅行规则消息 |
| `ConsentRecordId` | GDPR 同意记录 |
| `ErasureRequestId` | 数据删除请求 |
| `DelegatedKeyId` | 委托密钥（Agent 会话密钥） |
| `MpcTaskId` | MPC 签名任务 |
| `BlacklistEntryId` | 黑名单条目 |

每个 ID 自动实现: `Serialize/Deserialize`, `Display`, `From<Uuid>`, `sqlx::Type/Encode/Decode<Postgres>`。

### 5.2 资产与金额 (`types/asset.rs`)

```rust
enum Chain     { Solana, Ethereum, Polygon, Arbitrum, Base }
enum AssetSymbol { Usdc, Eth, Sol, Weth, Wsol }

struct Asset {
    symbol: AssetSymbol,
    chain: Chain,
    contract_address: Option<String>,
    decimals: u8,
}

struct Amount {
    value: Decimal,    // rust_decimal 高精度
    asset: AssetSymbol,
}
```

预置快捷构造: `Asset::usdc_solana()`, `Asset::sol()`, `Asset::usdc_ethereum()`, `Asset::eth()`。

### 5.3 参与者模型 (`types/participant.rs`)

**主从模型 (Master-Slave)**:

```rust
enum ParticipantKind { Human, Agent }

// 人类 (Master) — 持有 SBT，拥有独占的资金操作权
// 智能体 (Worker) — 通过委托密钥行动，只能签署 AcceptTask / SubmitResult
enum AgentPermission { AcceptTask, SubmitResult }

struct DelegatedKey {
    id: DelegatedKeyId,
    agent_id: ParticipantId,
    master_id: ParticipantId,
    public_key: String,           // Ed25519 公钥 (base58)
    permissions: Vec<AgentPermission>,
    expires_at: DateTime<Utc>,
    revoked: bool,
}

struct Participant {
    id: ParticipantId,
    kind: ParticipantKind,
    did: String,                  // 去中心化标识符 did:hmacs:<uuid>
    master_id: Option<ParticipantId>,  // Agent → 其 Human 主人
    sbt_verified: bool,           // 人类是否持有灵魂绑定代币 (KYC)
    blacklisted: bool,            // MPC 恶意行为黑名单
    capabilities: Vec<String>,
    ...
}
```

**关键规则**:
- `can_hold_funds()` — 仅 `Human` 且 `sbt_verified == true` 可持有资金
- Agent 尝试调用资金转移函数 → 触发 `AgentFundTransferForbidden` 硬拒绝

### 5.4 错误体系 (`error.rs`)

`HmacsError` 枚举包含 19 个变体，每个映射到 HTTP 状态码：

| 错误变体 | HTTP | 场景 |
|---------|------|------|
| `NotFound` | 404 | 实体不存在 |
| `AlreadyExists` | 409 | 重复创建 |
| `InvalidInput` | 400 | 参数校验失败 |
| `Unauthorized` | 401 | 未认证 |
| `Forbidden` | 403 | 权限不足 |
| `InsufficientBalance` | 422 | 余额不足 |
| `InvalidStateTransition` | 422 | 非法状态迁移 |
| `ComplianceBlocked` | 451 | 合规拦截 |
| `KycRequired` | 403 | KYC 等级不足 |
| `SanctionsMatch` | 451 | 制裁名单命中 |
| `TravelRuleViolation` | 422 | 旅行规则违规 |
| `AgentFundTransferForbidden` | 403 | Agent 尝试操作资金 |
| `FundsQuarantined` | 423 | 资金已被隔离 |
| `AgentBlacklisted` | 403 | Agent 已被永久拉黑 |
| `MpcError` | 500 | MPC 操作失败 |
| `SettlementError` | 502 | 链上结算失败 |
| `ChainError` | 502 | 区块链交互错误 |
| `DatabaseError` | 500 | 数据库错误 |
| `Internal` | 500 | 内部错误 |

### 5.5 分页 (`types/pagination.rs`)

```rust
struct PaginationParams { page: u64, per_page: u64 }
struct PaginatedResponse<T> { items: Vec<T>, total: u64, page: u64, per_page: u64, total_pages: u64 }
```

---

## 6. 身份与认证 — hmacs-identity

### 6.1 钱包签名认证 (`wallet_auth.rs`)

支持 Solana 钱包的 Ed25519 签名验证流程：

1. 服务端生成 `AuthChallenge`（包含域名、nonce、时间戳）
2. 客户端用钱包私钥签名 challenge message
3. `verify_solana_sign_in()` 验证签名：base58 解码地址/签名 → Ed25519 验证

### 6.2 JWT 令牌 (`jwt.rs`)

```rust
struct Claims {
    sub: String,
    participant_id: String,
    kind: String,           // "human" | "agent"
    iat: i64,
    exp: i64,
}
```

`create_token()` / `verify_token()` 使用 HS256 算法，默认 TTL 24 小时。

### 6.3 API Key (`api_key.rs`)

- 生成: `hmacs_` 前缀 + 32 字节随机数 hex 编码
- 存储: 仅存 SHA-256 哈希，原始密钥在创建时返回一次
- 验证: 计算输入哈希与存储哈希比对

### 6.4 委托密钥 (`delegation.rs`)

**`DelegationService`** 实现 Human → Agent 的权限委托：

| 方法 | 功能 |
|------|------|
| `create_delegated_key()` | Human 为 Agent 签发有时效的会话密钥 |
| `revoke_key()` | Human 撤销已签发的密钥 |
| `check_agent_permission()` | 验证 Agent 是否有指定操作权限 |
| `enforce_no_fund_transfer()` | 硬性拒绝 Agent 的资金操作请求 |
| `get_active_keys_for_agent()` | 获取 Agent 的所有有效密钥 |

**约束**: 仅 `ParticipantKind::Human` 可签发密钥；Agent 只能持有 `AcceptTask` 和 `SubmitResult` 权限。

---

## 7. 撮合引擎 — hmacs-engine

### 7.1 订单模型 (`order.rs`)

```rust
enum OrderSide   { Buy, Sell }
enum OrderType   { Market, Limit }
enum OrderStatus { Open, PartiallyFilled, Filled, Cancelled, Expired }

struct Order {
    id: OrderId,
    participant_id: ParticipantId,
    side: OrderSide,
    order_type: OrderType,
    asset: AssetSymbol,
    price: Decimal,
    quantity: Decimal,
    filled_quantity: Decimal,
    reference_id: Option<String>,   // 关联到 task_id / compute_lease_id
    ...
}

struct TradeExecution {
    buy_order_id: OrderId,
    sell_order_id: OrderId,
    price: Decimal,
    quantity: Decimal,
    ...
}
```

### 7.2 订单簿 (`orderbook.rs`)

`OrderBook` 使用 `BTreeMap<OrderBookKey, Order>` 实现价格-时间优先级排序：

- **买单 (Bids)**: 价格取反排序，最高价优先
- **卖单 (Asks)**: 自然排序，最低价优先
- 使用 `parking_lot::RwLock` 保证并发安全

`OrderBookManager` 使用 `DashMap<AssetSymbol, Arc<OrderBook>>` 管理多资产订单簿。

### 7.3 撮合逻辑 (`matcher.rs`)

`MatchingEngine::submit_order()` 流程：

1. 校验数量和价格有效性
2. 根据订单类型调用 `match_limit_order()` 或 `match_market_order()`
3. **限价单**: 从对手方取最优价，比较价格是否匹配，执行部分或全额成交
4. **市价单**: 直接吃对手方最优价，直到完全成交或深度耗尽
5. 未完全成交的限价单插入订单簿等待

---

## 8. 任务市场 — hmacs-task

### 8.1 任务状态机 (`state_machine.rs`)

```
         ┌─────────────────────────────────────────┐
         │                                         ▼
[Open] ──→ [Bidding] ──→ [Assigned] ──→ [InProgress] ──→ [Delivered]
  │                          │                               │    │
  │                          │                               ▼    ▼
  └──→ [Cancelled] ◄────────┘                        [Completed] [Disputed]
                                                          │         │
                                                          ▼         ▼
                                                      [Settled]  [Resolved]
                                                                    │
                                                                    ▼
                                                                [Settled]
```

`transition_to()` 方法强制执行有效转换规则，非法转换返回 `InvalidStateTransition` 错误。

### 8.2 任务模型 (`model.rs`)

```rust
struct Task {
    id: TaskId,
    creator_id: ParticipantId,
    title: String,
    description: String,
    skill_tags: Vec<String>,
    budget_asset: AssetSymbol,
    budget_amount: Decimal,
    status: TaskStatus,
    restriction: ParticipantRestriction,  // HumanOnly / AgentOnly / Any
    assigned_to: Option<ParticipantId>,
    deadline: Option<DateTime<Utc>>,
    deliverable_url: Option<String>,
    deliverable_notes: Option<String>,
    ...
}
```

### 8.3 竞标系统 (`bid.rs`)

```rust
enum BidStatus { Pending, Accepted, Rejected, Withdrawn }

struct Bid {
    id: BidId,
    task_id: TaskId,
    bidder_id: ParticipantId,
    amount: Decimal,
    proposal: String,
    estimated_duration_hours: Option<f64>,
    ...
}
```

### 8.4 任务服务 (`service.rs`)

`TaskService` 提供完整的任务生命周期管理：

| 方法 | 描述 |
|------|------|
| `create_task()` | 创建任务（Open 状态） |
| `place_bid()` | 提交竞标（Open → Bidding） |
| `accept_bid()` | 接受竞标（Bidding → Assigned），自动拒绝其他竞标 |
| `start_task()` | 开始工作（Assigned → InProgress） |
| `deliver_task()` | 提交交付物（InProgress → Delivered） |
| `approve_delivery()` | 确认交付（Delivered → Completed） |
| `dispute_delivery()` | 发起争议（Delivered → Disputed） |
| `cancel_task()` | 取消任务 |

---

## 9. 算力市场 — hmacs-compute

### 9.1 资源模型 (`resource.rs`)

```rust
struct ComputeResource {
    cpu_cores: u32,
    memory_gb: u32,
    gpu: Option<GpuInfo>,       // { model, vram_gb, count }
    bandwidth_mbps: Option<u32>,
    storage_gb: Option<u32>,
    region: Option<String>,
    status: ResourceStatus,     // Available / PartiallyLeased / FullyLeased / Offline
    tags: Vec<String>,
    ...
}
```

`ResourceFilter` 支持按 CPU/内存/GPU 显存/型号/区域筛选。

### 9.2 三种定价模式 (`pricing.rs`)

```rust
enum PricingMode {
    // 模式1: 总量出租 — 固定算力包，一口价
    BulkPackage { total_units: Decimal, unit_label: String, total_price: Decimal, asset: AssetSymbol },

    // 模式2: 按时出租 — 单位时间定价，按实际用量结算
    PerUnitTime { price_per_unit: Decimal, unit_label: String, asset: AssetSymbol, min/max_units },

    // 模式3: 自由定价 — 卖方自行标价，平台提供量价参考
    FreePrice { asking_price: Decimal, asset: AssetSymbol, unit_label: String, quantity: Decimal },
}
```

### 9.3 量价指导系统

`get_price_guidance()` 基于当前活跃 listing 计算：

```rust
struct PriceGuidance {
    average_price: Decimal,           // 均价
    median_price: Decimal,            // 中位数
    low_price / high_price: Decimal,  // 最低/最高
    recommended_range: (Decimal, Decimal),  // 推荐价格区间 (±15%)
    supply_count: u64,                // 供给数
    demand_count: u64,                // 需求数
    supply_demand_ratio: f64,         // 供需比
}
```

### 9.4 用量计量 (`metering.rs`)

`ComputeLease` 跟踪算力使用：

- `record_usage(units)` — 累加用量、计算总费用
- `remaining_units()` — 剩余可用量
- 当 `units_consumed >= max_units` 时自动标记 `LeaseStatus::Completed`

---

## 10. 内部账本 — hmacs-wallet

### 10.1 余额结构 (`balance.rs`)

```rust
struct Balance {
    wallet_id: WalletId,
    participant_id: ParticipantId,
    asset: AssetSymbol,
    available: Decimal,      // 可用余额
    frozen: Decimal,         // 已冻结（用于活跃任务/订单）
    quarantined: Decimal,    // 已隔离（风控触发，不可操作）
}
```

**三态模型**: `Available` ↔ `Frozen` ↔ `Quarantined`

### 10.2 账本流水 (`ledger.rs`)

```rust
enum LedgerEntryType {
    Deposit,                 // 充值
    Withdrawal,              // 提现
    Freeze,                  // 冻结
    Unfreeze,                // 解冻
    Transfer,                // 转账
    Fee,                     // 手续费
    Refund,                  // 退款
    Settlement,              // 结算
    Quarantine,              // 风控隔离
    Unquarantine,            // 解除隔离
    Slash,                   // 惩罚没收
    RobinHoodDistribution,   // 罗宾汉分配
}
```

所有操作产生不可篡改的 `LedgerEntry` 流水记录。

### 10.3 钱包服务 (`service.rs`)

| 方法 | 描述 |
|------|------|
| `deposit()` | 充值 → available 增加 |
| `withdraw()` | 提现 → available 减少（quarantined > 0 时阻止） |
| `freeze()` | 冻结 → available → frozen |
| `unfreeze()` | 解冻 → frozen → available |
| `quarantine()` | 风控隔离 → available → quarantined |
| `unquarantine()` | 管理员解除 → quarantined → available |
| `slash_and_distribute()` | 没收恶意 Agent 的 frozen 保证金并分配给诚实参与者 |
| `settle_transfer()` | 结算 → 从 A 的 frozen 转到 B 的 available |
| `enforce_human_only()` | Agent 调用资金函数时硬拒绝 |

**罗宾汉池 (Robin Hood Pool)**:
- 被没收的保证金**即时**平均分配给同一任务的诚实 Agent
- 若无诚实参与者或存在精度溢出，资金流入 `PUBLIC_FAUCET` 账户（用于补贴新开发者的 gas 费）
- **平台金库 (Treasury) 从罚没中获得的金额严格为零**

---

## 11. 链上结算 — hmacs-settlement

### 11.1 链抽象 Trait (`chain.rs`)

```rust
#[async_trait]
trait ChainAdapter: Send + Sync {
    fn chain(&self) -> Chain;
    async fn get_balance(&self, address, token_address) -> OnChainBalance;
    async fn send_token(&self, from_keypair, to, token, amount, decimals) -> ChainTxResult;
    async fn verify_transaction(&self, tx_hash) -> ChainTxResult;
    async fn watch_deposits(&self, watch_address, token, from_slot) -> Vec<DepositEvent>;
}
```

### 11.2 链适配器

| 适配器 | 支持链 | 状态 |
|--------|--------|------|
| `SolanaAdapter` | Solana (devnet/mainnet) | 骨架已就绪，待接入 solana-client |
| `EvmAdapter` | Ethereum, Polygon, Arbitrum, Base | 骨架已就绪，待接入 alloy |

### 11.3 充值/提现/对账

- **充值** (`deposit.rs`): `scan_deposits()` 扫描链上转入 → 生成 `DepositRecord` → Pending → Confirmed → Credited
- **提现** (`withdraw.rs`): `process_withdrawal()` → Processing → Submitted → Confirmed / Failed
- **对账** (`reconciliation.rs`): `reconcile()` 比较链上余额与内部账本，差异超过 0.000001 触发告警

---

## 12. 数据存储 — hmacs-storage

### 12.1 连接池 (`pool.rs`)

```rust
struct DbConfig {
    url: String,
    max_connections: u32,        // 默认 20
    min_connections: u32,        // 默认 2
    connect_timeout_secs: u64,   // 默认 10
    idle_timeout_secs: u64,      // 默认 300
}

async fn create_pool(config: &DbConfig) -> PgPool;
async fn run_migrations(pool: &PgPool);
```

### 12.2 Repository Trait (`repo.rs`)

```rust
#[async_trait]
trait Repository<T, Id, Create>: Send + Sync {
    async fn get_by_id(&self, id: Id) -> HmacsResult<T>;
    async fn create(&self, input: Create) -> HmacsResult<T>;
    async fn delete(&self, id: Id) -> HmacsResult<()>;
    async fn list(&self, params: &PaginationParams) -> HmacsResult<(Vec<T>, u64)>;
}
```

---

## 13. API 网关 — hmacs-api

### 13.1 应用状态 (`state.rs`)

```rust
struct AppState {
    task_service: Arc<TaskService>,
    compute_service: Arc<ComputeService>,
    wallet_service: Arc<WalletService>,
    matching_engine: Arc<MatchingEngine>,
    compliance_service: Arc<ComplianceService>,
    delegation_service: Arc<DelegationService>,
    mpc_service: Arc<MpcService>,
    jwt_config: Arc<JwtConfig>,
}
```

所有服务通过 `Arc` 共享，支持并发访问。

### 13.2 路由表

```
GET  /health                              → 健康检查
POST /api/v1/auth/wallet-sign-in          → 钱包签名登录

GET  /api/v1/tasks                        → 任务列表
POST /api/v1/tasks                        → 创建任务
GET  /api/v1/tasks/{id}                   → 任务详情
POST /api/v1/tasks/{id}/bids              → 提交竞标
GET  /api/v1/tasks/{id}/bids              → 竞标列表
POST /api/v1/tasks/{id}/accept-bid/{bid}  → 接受竞标
POST /api/v1/tasks/{id}/start             → 开始任务
POST /api/v1/tasks/{id}/deliver           → 提交交付
POST /api/v1/tasks/{id}/approve           → 确认交付
POST /api/v1/tasks/{id}/dispute           → 发起争议
POST /api/v1/tasks/{id}/cancel            → 取消任务

POST /api/v1/compute/resources            → 注册算力资源
GET  /api/v1/compute/resources            → 资源列表
POST /api/v1/compute/listings             → 创建上架
GET  /api/v1/compute/listings             → 上架列表
POST /api/v1/compute/listings/{id}/purchase → 购买租赁
GET  /api/v1/compute/leases/{id}          → 租赁详情
POST /api/v1/compute/leases/{id}/usage    → 上报用量
GET  /api/v1/compute/guidance             → 量价指导

GET  /api/v1/wallet/balances              → 余额查询
POST /api/v1/wallet/deposit               → 充值
POST /api/v1/wallet/withdraw              → 提现
GET  /api/v1/wallet/ledger                → 流水查询

POST /api/v1/compliance/kyc               → 提交 KYC
GET  /api/v1/compliance/kyc               → 查询 KYC 状态
POST /api/v1/compliance/kyc/check         → KYC 等级检查
POST /api/v1/compliance/screening         → 制裁筛查
GET  /api/v1/compliance/risk              → 风险评估结果
POST /api/v1/compliance/risk/assess       → 执行风险评估
GET  /api/v1/compliance/alerts            → 我的合规警报
GET  /api/v1/compliance/alerts/open       → 所有未处理警报
POST /api/v1/compliance/travel-rule/check → 旅行规则检查
POST /api/v1/compliance/consent           → 记录数据同意
POST /api/v1/compliance/erasure           → 数据删除请求
POST /api/v1/compliance/pre-check         → 交易前综合检查
GET  /api/v1/compliance/jurisdictions/{code} → 司法管辖区规则

WS   /ws                                  → WebSocket 实时推送
```

### 13.3 认证中间件 (`middleware.rs`)

提取 `Authorization: Bearer <JWT>` → 验证 → 注入 `AuthenticatedParticipant` 到请求扩展。

### 13.4 错误处理 (`error.rs`)

`ApiError` 包装 `HmacsError`，实现 `IntoResponse`：

```json
{
  "error": {
    "code": 404,
    "message": "Task with id abc123 not found"
  }
}
```

### 13.5 WebSocket (`ws.rs`)

支持的消息类型：

```rust
enum WsMessage {
    TaskUpdated { task_id, status },
    BidReceived { task_id, bid_id },
    LeaseUpdate { lease_id, units_consumed },
    BalanceChanged { asset, available, frozen },
    Ping / Pong,
}
```

---

## 14. Agent SDK — hmacs-sdk

### 14.1 客户端 (`client.rs`)

```rust
struct HmacsClient {
    config: HmacsClientConfig,  // endpoint, api_key, jwt_token, retries, timeout
}

impl HmacsClient {
    fn with_api_key(self, key: &str) -> Self;
    fn with_token(self, token: &str) -> Self;
    async fn connect(&self) -> HmacsResult<()>;
    // Task/Compute/Wallet 操作接口（待接入 gRPC）
}
```

### 14.2 认证 (`auth.rs`)

```rust
enum SdkAuth { ApiKey(String), JwtToken(String) }
fn sign_message_ed25519(keypair: &[u8; 64], message: &[u8]) -> [u8; 64];
```

---

## 15. 合规引擎 — hmacs-compliance

### 15.1 司法管辖区 (`jurisdiction.rs`)

| 参数 | US | EU | HK | SG | AE |
|------|-----|-----|-----|-----|-----|
| **监管机构** | FinCEN/SEC/CFTC | NCA | SFC | MAS | VARA |
| **牌照类型** | MSB | CASP (MiCA) | VASP License | MPI (PSA) | VASP (VARA) |
| **最低 KYC** | Basic | Standard | Standard | Standard | Standard |
| **旅行规则阈值** | $3,000 | €1,000 | $1,000 | SGD 1,500 | $1,000 |
| **大额报告** | $10,000 | €15,000 | HKD 120,000 | SGD 20,000 | AED 55,000 |
| **数据保护** | N/A | GDPR | PDPO | PDPA | UAE DPL |
| **数据保留期** | 5 年 | 5 年 | 6 年 | 5 年 | 5 年 |

### 15.2 KYC 分级 (`kyc.rs`)

| 等级 | 所需文件 |
|------|---------|
| None | 无 |
| Basic | 自拍+证件 |
| Standard | 护照 + 地址证明 + 自拍+证件 |
| Enhanced | 护照 + 地址证明 + 自拍+证件 + 资金来源 + 银行流水 |

等级支持比较运算: `Enhanced > Standard > Basic > None`

### 15.3 AML 筛查 (`aml.rs`)

筛查 8 个制裁名单: OFAC SDN, OFAC Consolidated, EU Consolidated, UN Sanctions, UK HMT, HK Designated, SG MAS, AE Local List。

每个司法管辖区有必查名单组合（如 US → OFAC SDN + OFAC Consolidated + UN）。

### 15.4 交易监控 (`monitoring.rs`)

5 条内置规则：

| 规则 | 触发条件 |
|------|---------|
| `LargeTransaction` | 金额 ≥ 大额报告阈值 |
| `Structuring` | 金额在阈值 90%-100% 之间（疑似拆分） |
| `RapidTransactions` | 窗口期内交易数 ≥ 10 |
| `DailyVolumeExceeded` | 日累计 > $50,000 |
| `NewAccountLargeTx` | 账龄 < 30 天且金额 ≥ 升级 KYC 阈值 |

### 15.5 旅行规则 (`travel_rule.rs`)

当转账金额超过管辖区阈值时，需收集并传递：
- **发起方信息**: 姓名、钱包地址、账户引用、地理地址（美国）
- **接收方信息**: 姓名、钱包地址、账户引用（欧盟）
- **VASP 信息**: 名称、LEI、管辖区

### 15.6 风险评分 (`risk.rs`)

5 因子评分模型（满分 100）：

| 因子 | 权重 | 计分逻辑 |
|------|------|---------|
| KYC 完整度 | 0-20 | Enhanced=0, Standard=5, Basic=12, None=20 |
| 地理风险 | 0-25 | 制裁国=25, 未知=10, 低风险国=0 |
| PEP 状态 | 0-20 | 是=20, 否=0 |
| 账户成熟度 | 0-15 | <7天=15, <30天=10, <90天=5, ≥90天=0 |
| 交易量 | 0-20 | >$100k=15, >$50k=10, >$10k=5, ≤$10k=0 |

风险等级: Low (<30) / Medium (30-59) / High (60+) / Prohibited (制裁命中)

### 15.7 GDPR (`gdpr.rs`)

- **同意管理**: 按数据类别(Identity/Contact/Financial/KycDocuments/Behavioral/Technical/WalletAddresses)和法律依据(Consent/ContractPerformance/LegalObligation/LegitimateInterest)记录
- **数据删除**: 评估每个数据类别是否可删除，考虑法定保留期、活跃交易、进行中调查

### 15.8 综合服务 (`service.rs`)

`ComplianceService` 聚合所有子模块，提供：
- `pre_transaction_check()` — 交易前一站式检查（KYC + 制裁 + 风险评分）
- `monitor_transaction()` — 交易后监控，生成警报
- `check_travel_rule()` — 检查是否需要旅行规则合规并验证信息完整性

---

## 16. MPC 子系统 — hmacs-mpc

### 16.1 MPC 任务状态机 (`state.rs`)

```
[Open] ──→ [Racing] ──→ [Aggregating] ──→ [Settled]
  ▲            │              │
  │            ▼              ▼
  └──── [Failed] ◄───────────┘
        (自动重试)
```

### 16.2 任务与份额类型 (`types.rs`)

```rust
struct MpcTask {
    id: MpcTaskId,
    state: MpcTaskState,
    threshold: usize,              // t (如 3)
    total_agents: usize,           // n (如 5)
    message: Vec<u8>,              // 待签名的消息
    participants: Vec<ParticipantId>,
    valid_shares: Vec<(ParticipantId, SignatureShare)>,
    malicious_agents: Vec<ParticipantId>,
    stake_amount: Decimal,         // 微额保证金 (如 0.1 USDC)
    bounty_per_agent: Decimal,     // 赏金/人
    final_signature: Option<Signature>,
    retry_count: u32,
}

struct SignatureShare {
    agent_id: ParticipantId,
    share_bytes: Vec<u8>,          // 64 字节 Ed25519
    vss_commitment: Vec<u8>,       // VSS 承诺值
}
```

### 16.3 冗余竞赛 (`racing.rs`)

`execute_frost_race()` 核心逻辑：

```rust
pub async fn execute_frost_race(
    share_rx: ShareReceiver,    // mpsc 通道接收份额
    message: &[u8],
    threshold: usize,
    total_agents: usize,
    timeout: Duration,
) -> Result<RaceResult, MpcError>
```

**执行流程**:
1. 启动 `tokio::select!` 循环，同时监听份额通道和超时
2. 每收到一个份额 → VSS 验证 → 有效则加入 `valid_shares`，无效则标记为恶意
3. 当 `valid_shares.len() >= threshold` → **立即跳出循环**
4. `drop(share_rx)` — 丢弃剩余的 pending future，迟到的 Agent 不受惩罚
5. 超时前未集齐 → 返回 `MpcError::Timeout`

### 16.4 VSS 验证 (`vss.rs`)

```rust
fn verify_share(share: &SignatureShare, message: &[u8]) -> VssResult {
    // 1. 份额长度必须为 64 字节
    // 2. VSS 承诺不能为空
    // 3. 承诺值前 4 字节 == SHA256(share_bytes || message)[0..4]
    //    (生产环境替换为完整的 Feldman/Pedersen VSS 验证)
}
```

### 16.5 MPC 服务 (`service.rs`)

| 方法 | 描述 |
|------|------|
| `create_task()` | 创建 t-of-n 签名任务（Open 状态） |
| `join_race()` | Agent 锁定微额保证金加入竞赛（检查黑名单） |
| `start_race()` | 启动竞赛，返回份额提交通道 |
| `settle_race()` | 处理竞赛结果：解锁诚实者保证金、分发赏金、没收恶意保证金 |
| `fail_and_retry()` | 标记失败、解锁所有保证金、生成新 TaskId 重试 |
| `blacklist_agent()` | 永久拉黑恶意 Agent |

**惩罚与分配流程**:
1. 检测到无效 VSS 份额 → 识别恶意 Agent
2. `slash_and_distribute()` → 没收 0.1 USDC 保证金
3. 保证金即时平分给该任务的所有诚实参与者
4. 无诚实参与者 → 转入 `PUBLIC_FAUCET`
5. 平台金库严格为 0

---

## 17. gRPC 协议定义

`proto/hmacs.proto` 定义 4 个服务，共 25 个 RPC 方法：

| 服务 | 方法数 | 主要操作 |
|------|--------|---------|
| `AuthService` | 3 | WalletSignIn, CreateApiKey, VerifyApiKey |
| `TaskService` | 11 | 完整任务生命周期 + 竞标管理 |
| `ComputeService` | 8 | 资源注册、上架、租赁、计量、量价指导 |
| `WalletService` | 3 | 余额查询、流水查询 |

所有金额字段使用 `string` 类型（避免浮点精度问题），时间使用自定义 `Timestamp` 消息。

---

## 18. 数据库 Schema

### 18.1 核心表 (`001_initial.sql`) — 15 张表

| 表 | 主要字段 | 索引 |
|----|---------|------|
| `participants` | id, kind, display_name, wallet_address, capabilities | kind, wallet_address |
| `api_keys` | id, participant_id, key_hash, label, revoked | participant_id, key_hash |
| `tasks` | id, creator_id, title, budget, status, assigned_to | creator_id, status, assigned_to |
| `bids` | id, task_id, bidder_id, amount, status | task_id, bidder_id |
| `compute_resources` | id, owner_id, cpu, memory, gpu(JSONB), status | owner_id, status |
| `compute_listings` | id, resource_id, seller_id, pricing(JSONB), active | seller_id, active |
| `compute_leases` | id, resource_id, buyer_id, seller_id, status | buyer_id, seller_id, status |
| `balances` | wallet_id, participant_id, asset, available, frozen | participant_id (UNIQUE: participant_id+asset) |
| `ledger_entries` | id, participant_id, entry_type, amount, direction | participant_id, entry_type, created_at |
| `settlement_deposits` | id, chain, tx_hash, status | status, tx_hash |
| `settlement_withdrawals` | id, chain, to_address, status | status |
| `orders` | id, participant_id, side, type, price, quantity, status | participant_id, status, asset+side |
| `trade_executions` | id, buy_order_id, sell_order_id, price, quantity | buy_order_id, sell_order_id, executed_at |
| `metering_events` | id, lease_id, units | lease_id |

所有金额字段类型: `DECIMAL(38, 18)`，时间字段: `TIMESTAMPTZ`。

### 18.2 合规表 (`002_compliance.sql`) — 9 张表

| 表 | 用途 |
|----|------|
| `kyc_records` | KYC 审核记录（加密存储敏感字段） |
| `kyc_documents` | KYC 文件引用（仅存哈希和加密存储路径） |
| `screening_records` | 制裁/AML 筛查记录 |
| `screening_matches` | 筛查命中详情 |
| `compliance_alerts` | 合规警报（类型、严重度、状态） |
| `travel_rule_messages` | 旅行规则消息（发起方/接收方/VASP 信息） |
| `risk_assessments` | 风险评估结果（评分、因子 JSONB） |
| `consent_records` | GDPR 同意记录 |
| `erasure_requests` | 数据删除请求及处理状态 |

---

## 19. 配置系统

使用 `config-rs` + TOML 格式，支持分环境覆盖：

```toml
[server]
bind_addr = "0.0.0.0:8080"

[database]
url = "postgres://hmacs:hmacs@localhost:5432/hmacs"
max_connections = 20

[redis]
url = "redis://localhost:6379"

[auth]
jwt_secret = "hmacs-dev-secret-change-in-production"
jwt_token_ttl_hours = 24

[settlement.solana]
rpc_url = "https://api.devnet.solana.com"

[settlement.ethereum]
rpc_url = "https://rpc.sepolia.org"
```

生产环境敏感配置通过环境变量注入: `HMACS_DATABASE_URL`, `HMACS_JWT_SECRET`。

---

## 20. 测试覆盖

**总计: 97 个单元测试, 0 个失败, 0 clippy 警告**

| Crate | 测试数 | 覆盖场景 |
|-------|--------|---------|
| `hmacs-core` | 5 | ID 创建/显示/往返转换, 金额显示, 资产预设 |
| `hmacs-identity` | 9 | JWT 往返, API Key 生成/验证, 钱包签名, 委托密钥 CRUD, Agent 权限拒绝 |
| `hmacs-engine` | 7 | 买卖排序, 价差计算, 精确/部分成交, 价格优先级 |
| `hmacs-task` | 5 | 完整生命周期, 争议路径, 无效转换, 状态转换规则 |
| `hmacs-compute` | 7 | 资源注册/筛选, 上架/租赁/计量, 量价指导, 自动完成 |
| `hmacs-wallet` | 8 | 充值/冻结/解冻/结算/流水, 隔离/解除隔离, 惩罚分配, 水龙头兜底 |
| `hmacs-compliance` | 39 | 各管辖区阈值, KYC 分级, 制裁筛查, 交易监控5规则, 旅行规则, 风险评分, GDPR删除, 综合前置检查 |
| `hmacs-mpc` | 17 | 状态机转换, 竞赛达标/超时/恶意检测, VSS 验证(有效/空/错误承诺/错误长度), 任务创建/加入/重试, 黑名单 |

---

## 21. 技术选型表

| 层面 | 选型 | 理由 |
|------|------|------|
| 语言 | Rust 1.94 | 内存安全、零成本抽象、近裸机性能 |
| Web 框架 | Axum 0.7 | Tokio 生态原生、Tower 中间件、WebSocket 内置 |
| gRPC | Tonic 0.12 | Rust 原生 gRPC, Agent SDK 首选协议 |
| 数据库 | PostgreSQL + SQLx 0.8 | 异步、编译期 SQL 检查、JSONB 支持 |
| 缓存 | Redis | 撮合队列、WebSocket pub/sub |
| 序列化 | serde + protobuf | REST → JSON, gRPC → proto |
| 密码学 | ed25519-dalek 2.x | Ed25519 签名验证（Solana 原生） |
| 认证 | jsonwebtoken 9.x | HS256 JWT |
| 精度 | rust_decimal | 38 位精度，避免浮点误差 |
| 区块链 | solana-sdk 2.x, alloy 0.5 | 多链适配 |
| 并发 | parking_lot, dashmap | 高性能读写锁、无锁并发 HashMap |
| 日志 | tracing + tracing-subscriber | 结构化日志、链路追踪 |
| 错误 | thiserror + anyhow | 类型化领域错误 + 万能错误 |
| 异步 | tokio (full) + futures | 异步运行时 + Stream/Select |
| 配置 | config-rs + TOML | 分环境配置、环境变量覆盖 |

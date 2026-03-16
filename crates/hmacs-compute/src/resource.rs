use chrono::{DateTime, Utc};
use hmacs_core::{ComputeResourceId, ParticipantId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    Available,
    PartiallyLeased,
    FullyLeased,
    Offline,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub model: String,
    pub vram_gb: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResource {
    pub id: ComputeResourceId,
    pub owner_id: ParticipantId,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub gpu: Option<GpuInfo>,
    pub bandwidth_mbps: Option<u32>,
    pub storage_gb: Option<u32>,
    pub status: ResourceStatus,
    /// Available time windows (start, end)
    pub available_from: Option<DateTime<Utc>>,
    pub available_until: Option<DateTime<Utc>>,
    pub region: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResource {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub gpu: Option<GpuInfo>,
    pub bandwidth_mbps: Option<u32>,
    pub storage_gb: Option<u32>,
    pub available_from: Option<DateTime<Utc>>,
    pub available_until: Option<DateTime<Utc>>,
    pub region: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceFilter {
    pub min_cpu_cores: Option<u32>,
    pub min_memory_gb: Option<u32>,
    pub min_gpu_vram_gb: Option<u32>,
    pub gpu_model: Option<String>,
    pub region: Option<String>,
    pub status: Option<ResourceStatus>,
}

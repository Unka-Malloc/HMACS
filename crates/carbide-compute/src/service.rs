use crate::metering::{ComputeLease, LeaseStatus, MeteringEvent};
use crate::pricing::{ComputeListing, CreateListing, PriceGuidance, PricingMode};
use crate::resource::{ComputeResource, RegisterResource, ResourceFilter, ResourceStatus};
use chrono::Utc;
use carbide_core::{
    AssetSymbol, ComputeLeaseId, ComputeResourceId, CarbideError, CarbideResult, ParticipantId,
};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

pub struct ComputeService {
    resources: RwLock<HashMap<ComputeResourceId, ComputeResource>>,
    listings: RwLock<HashMap<uuid::Uuid, ComputeListing>>,
    leases: RwLock<HashMap<ComputeLeaseId, ComputeLease>>,
    metering_log: RwLock<Vec<MeteringEvent>>,
}

impl ComputeService {
    pub fn new() -> Self {
        Self {
            resources: RwLock::new(HashMap::new()),
            listings: RwLock::new(HashMap::new()),
            leases: RwLock::new(HashMap::new()),
            metering_log: RwLock::new(Vec::new()),
        }
    }

    pub fn register_resource(
        &self,
        owner_id: ParticipantId,
        input: RegisterResource,
    ) -> CarbideResult<ComputeResource> {
        let now = Utc::now();
        let resource = ComputeResource {
            id: ComputeResourceId::new(),
            owner_id,
            cpu_cores: input.cpu_cores,
            memory_gb: input.memory_gb,
            gpu: input.gpu,
            bandwidth_mbps: input.bandwidth_mbps,
            storage_gb: input.storage_gb,
            status: ResourceStatus::Available,
            available_from: input.available_from,
            available_until: input.available_until,
            region: input.region,
            tags: input.tags,
            created_at: now,
            updated_at: now,
        };
        self.resources.write().insert(resource.id, resource.clone());
        Ok(resource)
    }

    pub fn get_resource(&self, id: ComputeResourceId) -> CarbideResult<ComputeResource> {
        self.resources
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| CarbideError::not_found("ComputeResource", id))
    }

    pub fn list_resources(&self, filter: &ResourceFilter) -> Vec<ComputeResource> {
        self.resources
            .read()
            .values()
            .filter(|r| {
                if let Some(min_cpu) = filter.min_cpu_cores {
                    if r.cpu_cores < min_cpu {
                        return false;
                    }
                }
                if let Some(min_mem) = filter.min_memory_gb {
                    if r.memory_gb < min_mem {
                        return false;
                    }
                }
                if let Some(min_vram) = filter.min_gpu_vram_gb {
                    match &r.gpu {
                        Some(gpu) if gpu.vram_gb >= min_vram => {}
                        _ => return false,
                    }
                }
                if let Some(ref model) = filter.gpu_model {
                    match &r.gpu {
                        Some(gpu) if gpu.model.to_lowercase().contains(&model.to_lowercase()) => {}
                        _ => return false,
                    }
                }
                if let Some(ref region) = filter.region {
                    if r.region.as_ref() != Some(region) {
                        return false;
                    }
                }
                if let Some(status) = filter.status {
                    if r.status != status {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub fn create_listing(
        &self,
        seller_id: ParticipantId,
        input: CreateListing,
    ) -> CarbideResult<ComputeListing> {
        let resource = self.get_resource(input.resource_id)?;
        if resource.owner_id != seller_id {
            return Err(CarbideError::Forbidden(
                "Only the resource owner can create listings".into(),
            ));
        }

        let now = Utc::now();
        let listing = ComputeListing {
            id: uuid::Uuid::new_v4(),
            resource_id: input.resource_id,
            seller_id,
            pricing: input.pricing,
            active: true,
            created_at: now,
            updated_at: now,
        };
        self.listings.write().insert(listing.id, listing.clone());
        Ok(listing)
    }

    pub fn list_active_listings(&self) -> Vec<ComputeListing> {
        self.listings
            .read()
            .values()
            .filter(|l| l.active)
            .cloned()
            .collect()
    }

    pub fn purchase_lease(
        &self,
        buyer_id: ParticipantId,
        listing_id: uuid::Uuid,
        units: Option<Decimal>,
    ) -> CarbideResult<ComputeLease> {
        let listings = self.listings.read();
        let listing = listings
            .get(&listing_id)
            .ok_or_else(|| CarbideError::not_found("ComputeListing", listing_id))?;

        if !listing.active {
            return Err(CarbideError::InvalidInput("Listing is not active".into()));
        }

        let (price_per_unit, unit_label, max_units, asset) = match listing.pricing.clone() {
            PricingMode::BulkPackage {
                total_units,
                unit_label,
                total_price,
                asset,
            } => {
                let ppu = if total_units.is_zero() {
                    Decimal::ZERO
                } else {
                    total_price / total_units
                };
                (ppu, unit_label, Some(total_units), asset)
            }
            PricingMode::PerUnitTime {
                price_per_unit,
                unit_label,
                asset,
                max_units,
                ..
            } => (price_per_unit, unit_label, max_units, asset),
            PricingMode::FreePrice {
                asking_price,
                asset,
                unit_label,
                quantity,
            } => {
                let ppu = if quantity.is_zero() {
                    Decimal::ZERO
                } else {
                    asking_price / quantity
                };
                (ppu, unit_label, Some(quantity), asset)
            }
        };

        let effective_max = units.or(max_units);

        let now = Utc::now();
        let lease = ComputeLease {
            id: ComputeLeaseId::new(),
            resource_id: listing.resource_id,
            listing_id: listing.id,
            buyer_id,
            seller_id: listing.seller_id,
            asset,
            price_per_unit,
            unit_label,
            units_consumed: Decimal::ZERO,
            max_units: effective_max,
            total_cost: Decimal::ZERO,
            status: LeaseStatus::Active,
            started_at: now,
            last_metered_at: now,
            ended_at: None,
        };

        self.leases.write().insert(lease.id, lease.clone());
        Ok(lease)
    }

    pub fn report_usage(
        &self,
        lease_id: ComputeLeaseId,
        units: Decimal,
        reporter_id: ParticipantId,
    ) -> CarbideResult<ComputeLease> {
        let mut leases = self.leases.write();
        let lease = leases
            .get_mut(&lease_id)
            .ok_or_else(|| CarbideError::not_found("ComputeLease", lease_id))?;

        if lease.seller_id != reporter_id {
            return Err(CarbideError::Forbidden(
                "Only the resource provider can report usage".into(),
            ));
        }

        if lease.status != LeaseStatus::Active {
            return Err(CarbideError::InvalidInput("Lease is not active".into()));
        }

        lease.record_usage(units);

        self.metering_log.write().push(MeteringEvent {
            lease_id,
            units,
            recorded_at: Utc::now(),
        });

        Ok(lease.clone())
    }

    pub fn get_lease(&self, lease_id: ComputeLeaseId) -> CarbideResult<ComputeLease> {
        self.leases
            .read()
            .get(&lease_id)
            .cloned()
            .ok_or_else(|| CarbideError::not_found("ComputeLease", lease_id))
    }

    /// Calculate price guidance for a given unit type.
    pub fn get_price_guidance(
        &self,
        unit_label: &str,
        asset: AssetSymbol,
    ) -> PriceGuidance {
        let listings = self.listings.read();
        let mut prices: Vec<Decimal> = listings
            .values()
            .filter(|l| l.active && l.pricing.asset() == asset)
            .filter_map(|l| {
                let ppu = l.pricing.effective_price_per_unit()?;
                let label = match &l.pricing {
                    PricingMode::BulkPackage { unit_label, .. } => unit_label,
                    PricingMode::PerUnitTime { unit_label, .. } => unit_label,
                    PricingMode::FreePrice { unit_label, .. } => unit_label,
                };
                if label == unit_label {
                    Some(ppu)
                } else {
                    None
                }
            })
            .collect();

        prices.sort();

        let supply_count = prices.len() as u64;
        let demand_count = self
            .leases
            .read()
            .values()
            .filter(|l| l.status == LeaseStatus::Active && l.unit_label == unit_label)
            .count() as u64;

        let (avg, median, low, high) = if prices.is_empty() {
            (dec!(0), dec!(0), dec!(0), dec!(0))
        } else {
            let sum: Decimal = prices.iter().copied().sum();
            let count = Decimal::from(prices.len() as u64);
            let avg = sum / count;
            let median = prices[prices.len() / 2];
            let low = prices[0];
            let high = *prices.last().unwrap();
            (avg, median, low, high)
        };

        let ratio = if demand_count == 0 {
            if supply_count == 0 {
                1.0
            } else {
                f64::INFINITY
            }
        } else {
            supply_count as f64 / demand_count as f64
        };

        let margin = avg * dec!(0.15);
        let diff = avg - margin;
        let recommended_low = if diff > low { diff } else { low };
        let recommended_high = avg + margin;

        PriceGuidance {
            unit_label: unit_label.to_string(),
            asset,
            average_price: avg,
            median_price: median,
            low_price: low,
            high_price: high,
            recommended_range: (recommended_low, recommended_high),
            supply_count,
            demand_count,
            supply_demand_ratio: ratio,
            computed_at: Utc::now(),
        }
    }
}

impl Default for ComputeService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::GpuInfo;

    #[test]
    fn test_register_and_list_resource() {
        let svc = ComputeService::new();
        let owner = ParticipantId::new();
        let resource = svc
            .register_resource(
                owner,
                RegisterResource {
                    cpu_cores: 16,
                    memory_gb: 64,
                    gpu: Some(GpuInfo {
                        model: "A100".into(),
                        vram_gb: 80,
                        count: 1,
                    }),
                    bandwidth_mbps: Some(1000),
                    storage_gb: Some(500),
                    available_from: None,
                    available_until: None,
                    region: Some("us-east".into()),
                    tags: vec!["ml".into()],
                },
            )
            .unwrap();

        let found = svc.list_resources(&ResourceFilter {
            min_cpu_cores: Some(8),
            min_memory_gb: None,
            min_gpu_vram_gb: Some(40),
            gpu_model: Some("A100".into()),
            region: None,
            status: None,
        });
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, resource.id);
    }

    #[test]
    fn test_listing_and_lease_lifecycle() {
        let svc = ComputeService::new();
        let seller = ParticipantId::new();
        let buyer = ParticipantId::new();

        let resource = svc
            .register_resource(
                seller,
                RegisterResource {
                    cpu_cores: 8,
                    memory_gb: 32,
                    gpu: None,
                    bandwidth_mbps: None,
                    storage_gb: None,
                    available_from: None,
                    available_until: None,
                    region: None,
                    tags: vec![],
                },
            )
            .unwrap();

        let listing = svc
            .create_listing(
                seller,
                CreateListing {
                    resource_id: resource.id,
                    pricing: PricingMode::PerUnitTime {
                        price_per_unit: dec!(1),
                        unit_label: "CPU-hour".into(),
                        asset: AssetSymbol::Usdc,
                        min_units: None,
                        max_units: None,
                    },
                },
            )
            .unwrap();

        let lease = svc
            .purchase_lease(buyer, listing.id, Some(dec!(10)))
            .unwrap();
        assert_eq!(lease.status, LeaseStatus::Active);

        let lease = svc.report_usage(lease.id, dec!(3), seller).unwrap();
        assert_eq!(lease.units_consumed, dec!(3));
        assert_eq!(lease.total_cost, dec!(3));
    }

    #[test]
    fn test_price_guidance() {
        let svc = ComputeService::new();
        let seller = ParticipantId::new();

        for price in [dec!(0.5), dec!(0.7), dec!(0.9), dec!(1.1), dec!(1.3)] {
            let res = svc
                .register_resource(
                    seller,
                    RegisterResource {
                        cpu_cores: 8,
                        memory_gb: 32,
                        gpu: None,
                        bandwidth_mbps: None,
                        storage_gb: None,
                        available_from: None,
                        available_until: None,
                        region: None,
                        tags: vec![],
                    },
                )
                .unwrap();
            svc.create_listing(
                seller,
                CreateListing {
                    resource_id: res.id,
                    pricing: PricingMode::PerUnitTime {
                        price_per_unit: price,
                        unit_label: "GPU-hour".into(),
                        asset: AssetSymbol::Usdc,
                        min_units: None,
                        max_units: None,
                    },
                },
            )
            .unwrap();
        }

        let guidance = svc.get_price_guidance("GPU-hour", AssetSymbol::Usdc);
        assert!(guidance.average_price > Decimal::ZERO);
        assert!(guidance.recommended_range.0 <= guidance.recommended_range.1);
        assert_eq!(guidance.supply_count, 5);
    }
}

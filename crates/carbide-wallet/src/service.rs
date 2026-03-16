use crate::balance::Balance;
use crate::ledger::{LedgerEntry, LedgerEntryType};
use chrono::Utc;
use carbide_core::{
    AssetSymbol, CarbideError, CarbideResult, ParticipantId, ParticipantKind, TransactionId, WalletId,
};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::warn;

type BalanceKey = (ParticipantId, AssetSymbol);

/// Well-known account for subsidizing gas fees for new developers.
/// The platform treasury must never accumulate slashed funds — they go here.
pub const PUBLIC_FAUCET_LABEL: &str = "PUBLIC_FAUCET";

pub struct WalletService {
    balances: RwLock<HashMap<BalanceKey, Balance>>,
    ledger: RwLock<Vec<LedgerEntry>>,
    /// The public faucet account ID (set on construction)
    pub faucet_account: ParticipantId,
}

impl WalletService {
    pub fn new() -> Self {
        Self {
            balances: RwLock::new(HashMap::new()),
            ledger: RwLock::new(Vec::new()),
            faucet_account: ParticipantId::new(),
        }
    }

    fn new_balance(participant_id: ParticipantId, asset: AssetSymbol) -> Balance {
        Balance {
            wallet_id: WalletId::new(),
            participant_id,
            asset,
            available: Decimal::ZERO,
            frozen: Decimal::ZERO,
            quarantined: Decimal::ZERO,
            updated_at: Utc::now(),
        }
    }

    pub fn get_balance(&self, participant_id: ParticipantId, asset: AssetSymbol) -> Balance {
        let key = (participant_id, asset);
        self.balances
            .read()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Self::new_balance(participant_id, asset))
    }

    pub fn get_all_balances(&self, participant_id: ParticipantId) -> Vec<Balance> {
        self.balances
            .read()
            .iter()
            .filter(|((pid, _), _)| *pid == participant_id)
            .map(|(_, b): (&BalanceKey, &Balance)| b.clone())
            .collect()
    }

    /// Enforce the master-slave rule: agents cannot call fund-transfer functions.
    pub fn enforce_human_only(&self, caller_kind: ParticipantKind) -> CarbideResult<()> {
        if caller_kind == ParticipantKind::Agent {
            return Err(CarbideError::AgentFundTransferForbidden);
        }
        Ok(())
    }

    pub fn deposit(
        &self,
        participant_id: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
        reference_id: Option<String>,
    ) -> CarbideResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(CarbideError::InvalidInput(
                "Deposit amount must be positive".into(),
            ));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances
            .entry(key)
            .or_insert_with(|| Self::new_balance(participant_id, asset));

        balance.available += amount;
        balance.updated_at = Utc::now();

        self.record_entry(
            participant_id,
            LedgerEntryType::Deposit,
            asset,
            amount,
            1,
            balance.available,
            reference_id,
            Some("deposit".to_string()),
            None,
        );

        Ok(balance.clone())
    }

    pub fn withdraw(
        &self,
        participant_id: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
        reference_id: Option<String>,
    ) -> CarbideResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(CarbideError::InvalidInput(
                "Withdrawal amount must be positive".into(),
            ));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances.get_mut(&key).ok_or_else(|| {
            CarbideError::InsufficientBalance {
                needed: format!("{} {}", amount, asset),
                available: format!("0 {}", asset),
            }
        })?;

        if balance.has_quarantined() {
            return Err(CarbideError::FundsQuarantined(
                "Cannot withdraw while funds are quarantined".into(),
            ));
        }

        if !balance.can_spend(amount) {
            return Err(CarbideError::InsufficientBalance {
                needed: format!("{} {}", amount, asset),
                available: format!("{} {}", balance.available, asset),
            });
        }

        balance.available -= amount;
        balance.updated_at = Utc::now();

        self.record_entry(
            participant_id,
            LedgerEntryType::Withdrawal,
            asset,
            amount,
            -1,
            balance.available,
            reference_id,
            Some("withdrawal".to_string()),
            None,
        );

        Ok(balance.clone())
    }

    pub fn freeze(
        &self,
        participant_id: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
        reference_id: Option<String>,
    ) -> CarbideResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(CarbideError::InvalidInput(
                "Freeze amount must be positive".into(),
            ));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances.get_mut(&key).ok_or_else(|| {
            CarbideError::InsufficientBalance {
                needed: format!("{} {}", amount, asset),
                available: format!("0 {}", asset),
            }
        })?;

        if !balance.can_spend(amount) {
            return Err(CarbideError::InsufficientBalance {
                needed: format!("{} {}", amount, asset),
                available: format!("{} {}", balance.available, asset),
            });
        }

        balance.available -= amount;
        balance.frozen += amount;
        balance.updated_at = Utc::now();

        self.record_entry(
            participant_id,
            LedgerEntryType::Freeze,
            asset,
            amount,
            0,
            balance.available,
            reference_id,
            Some("freeze".to_string()),
            None,
        );

        Ok(balance.clone())
    }

    pub fn unfreeze(
        &self,
        participant_id: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
        reference_id: Option<String>,
    ) -> CarbideResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(CarbideError::InvalidInput(
                "Unfreeze amount must be positive".into(),
            ));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances
            .get_mut(&key)
            .ok_or_else(|| CarbideError::not_found("Balance", participant_id))?;

        if !balance.can_unfreeze(amount) {
            return Err(CarbideError::InvalidInput(format!(
                "Cannot unfreeze {} {}: only {} frozen",
                amount, asset, balance.frozen
            )));
        }

        balance.frozen -= amount;
        balance.available += amount;
        balance.updated_at = Utc::now();

        self.record_entry(
            participant_id,
            LedgerEntryType::Unfreeze,
            asset,
            amount,
            0,
            balance.available,
            reference_id,
            Some("unfreeze".to_string()),
            None,
        );

        Ok(balance.clone())
    }

    /// Atomically move funds from Available to Quarantined.
    /// Triggered by the risk/compliance middleware on AML alerts.
    pub fn quarantine(
        &self,
        participant_id: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
        reason: String,
    ) -> CarbideResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(CarbideError::InvalidInput(
                "Quarantine amount must be positive".into(),
            ));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances
            .get_mut(&key)
            .ok_or_else(|| CarbideError::not_found("Balance", participant_id))?;

        if !balance.can_spend(amount) {
            return Err(CarbideError::InsufficientBalance {
                needed: format!("{} {}", amount, asset),
                available: format!("{} {}", balance.available, asset),
            });
        }

        warn!(
            participant = %participant_id,
            amount = %amount,
            asset = %asset,
            reason = %reason,
            "Funds quarantined"
        );

        balance.available -= amount;
        balance.quarantined += amount;
        balance.updated_at = Utc::now();

        self.record_entry(
            participant_id,
            LedgerEntryType::Quarantine,
            asset,
            amount,
            0,
            balance.available,
            None,
            Some("quarantine".to_string()),
            Some(reason),
        );

        Ok(balance.clone())
    }

    /// Release quarantined funds back to available (admin action only).
    pub fn unquarantine(
        &self,
        participant_id: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
    ) -> CarbideResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(CarbideError::InvalidInput(
                "Unquarantine amount must be positive".into(),
            ));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances
            .get_mut(&key)
            .ok_or_else(|| CarbideError::not_found("Balance", participant_id))?;

        if balance.quarantined < amount {
            return Err(CarbideError::InvalidInput(format!(
                "Cannot unquarantine {} {}: only {} quarantined",
                amount, asset, balance.quarantined
            )));
        }

        balance.quarantined -= amount;
        balance.available += amount;
        balance.updated_at = Utc::now();

        self.record_entry(
            participant_id,
            LedgerEntryType::Unquarantine,
            asset,
            amount,
            0,
            balance.available,
            None,
            Some("unquarantine".to_string()),
            Some("Admin clearance".to_string()),
        );

        Ok(balance.clone())
    }

    /// Slash a participant's frozen micro-stake and distribute to honest agents
    /// (Robin Hood pool). The platform treasury receives exactly zero.
    pub fn slash_and_distribute(
        &self,
        malicious_agent: ParticipantId,
        asset: AssetSymbol,
        stake_amount: Decimal,
        honest_agents: &[ParticipantId],
        reference_id: Option<String>,
    ) -> CarbideResult<()> {
        if stake_amount <= Decimal::ZERO {
            return Err(CarbideError::InvalidInput(
                "Slash amount must be positive".into(),
            ));
        }

        let malicious_key = (malicious_agent, asset);
        let mut balances = self.balances.write();

        // Confiscate from frozen
        {
            let balance = balances
                .get_mut(&malicious_key)
                .ok_or_else(|| CarbideError::not_found("Balance", malicious_agent))?;
            if !balance.can_unfreeze(stake_amount) {
                return Err(CarbideError::InsufficientBalance {
                    needed: format!("{} {} (frozen)", stake_amount, asset),
                    available: format!("{} {} (frozen)", balance.frozen, asset),
                });
            }
            balance.frozen -= stake_amount;
            balance.updated_at = Utc::now();
        }

        self.record_entry(
            malicious_agent,
            LedgerEntryType::Slash,
            asset,
            stake_amount,
            -1,
            balances
                .get(&malicious_key)
                .map(|b| b.available)
                .unwrap_or_default(),
            reference_id.clone(),
            Some("slash".to_string()),
            Some("Malicious MPC share — stake confiscated".to_string()),
        );

        // Distribute to honest agents, or to faucet if none
        let recipients: Vec<ParticipantId> = if honest_agents.is_empty() {
            vec![self.faucet_account]
        } else {
            honest_agents.to_vec()
        };

        let share = stake_amount / Decimal::from(recipients.len() as u64);
        let mut distributed = Decimal::ZERO;

        for (i, recipient) in recipients.iter().enumerate() {
            let amount = if i == recipients.len() - 1 {
                stake_amount - distributed
            } else {
                share
            };
            distributed += amount;

            let rkey = (*recipient, asset);
            let bal = balances
                .entry(rkey)
                .or_insert_with(|| Self::new_balance(*recipient, asset));
            bal.available += amount;
            bal.updated_at = Utc::now();

            self.record_entry(
                *recipient,
                LedgerEntryType::RobinHoodDistribution,
                asset,
                amount,
                1,
                bal.available,
                reference_id.clone(),
                Some("robin_hood".to_string()),
                Some("Time-loss compensation from slashed stake".to_string()),
            );
        }

        Ok(())
    }

    /// Transfer frozen funds from one participant to another (settlement).
    pub fn settle_transfer(
        &self,
        from: ParticipantId,
        to: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
        reference_id: Option<String>,
    ) -> CarbideResult<()> {
        if amount <= Decimal::ZERO {
            return Err(CarbideError::InvalidInput(
                "Transfer amount must be positive".into(),
            ));
        }

        let from_key = (from, asset);
        let to_key = (to, asset);

        let mut balances = self.balances.write();

        {
            let from_balance = balances
                .get_mut(&from_key)
                .ok_or_else(|| CarbideError::not_found("Balance", from))?;
            if !from_balance.can_unfreeze(amount) {
                return Err(CarbideError::InsufficientBalance {
                    needed: format!("{} {} (frozen)", amount, asset),
                    available: format!("{} {} (frozen)", from_balance.frozen, asset),
                });
            }
            from_balance.frozen -= amount;
            from_balance.updated_at = Utc::now();
        }

        let to_balance = balances
            .entry(to_key)
            .or_insert_with(|| Self::new_balance(to, asset));
        to_balance.available += amount;
        to_balance.updated_at = Utc::now();

        let from_available = balances
            .get(&from_key)
            .map(|b| b.available)
            .unwrap_or_default();
        let to_available = balances
            .get(&to_key)
            .map(|b| b.available)
            .unwrap_or_default();

        let ref_id = reference_id.clone();
        self.record_entry(
            from,
            LedgerEntryType::Settlement,
            asset,
            amount,
            -1,
            from_available,
            ref_id,
            Some("settlement".to_string()),
            Some(format!("Transfer to {}", to)),
        );
        self.record_entry(
            to,
            LedgerEntryType::Settlement,
            asset,
            amount,
            1,
            to_available,
            reference_id,
            Some("settlement".to_string()),
            Some(format!("Transfer from {}", from)),
        );

        Ok(())
    }

    pub fn get_ledger_entries(&self, participant_id: ParticipantId) -> Vec<LedgerEntry> {
        self.ledger
            .read()
            .iter()
            .filter(|e| e.participant_id == participant_id)
            .cloned()
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn record_entry(
        &self,
        participant_id: ParticipantId,
        entry_type: LedgerEntryType,
        asset: AssetSymbol,
        amount: Decimal,
        direction: i8,
        balance_after: Decimal,
        reference_id: Option<String>,
        reference_type: Option<String>,
        description: Option<String>,
    ) {
        self.ledger.write().push(LedgerEntry {
            id: TransactionId::new(),
            participant_id,
            entry_type,
            asset,
            amount,
            direction,
            balance_after,
            reference_id,
            reference_type,
            description,
            created_at: Utc::now(),
        });
    }
}

impl Default for WalletService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_deposit_and_balance() {
        let svc = WalletService::new();
        let pid = ParticipantId::new();
        svc.deposit(pid, AssetSymbol::Usdc, dec!(100), None)
            .unwrap();
        let bal = svc.get_balance(pid, AssetSymbol::Usdc);
        assert_eq!(bal.available, dec!(100));
        assert_eq!(bal.frozen, dec!(0));
        assert_eq!(bal.quarantined, dec!(0));
    }

    #[test]
    fn test_freeze_unfreeze() {
        let svc = WalletService::new();
        let pid = ParticipantId::new();
        svc.deposit(pid, AssetSymbol::Usdc, dec!(100), None)
            .unwrap();
        svc.freeze(pid, AssetSymbol::Usdc, dec!(30), None).unwrap();
        let bal = svc.get_balance(pid, AssetSymbol::Usdc);
        assert_eq!(bal.available, dec!(70));
        assert_eq!(bal.frozen, dec!(30));

        svc.unfreeze(pid, AssetSymbol::Usdc, dec!(10), None)
            .unwrap();
        let bal = svc.get_balance(pid, AssetSymbol::Usdc);
        assert_eq!(bal.available, dec!(80));
        assert_eq!(bal.frozen, dec!(20));
    }

    #[test]
    fn test_insufficient_balance() {
        let svc = WalletService::new();
        let pid = ParticipantId::new();
        svc.deposit(pid, AssetSymbol::Usdc, dec!(50), None)
            .unwrap();
        let result = svc.withdraw(pid, AssetSymbol::Usdc, dec!(100), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_settle_transfer() {
        let svc = WalletService::new();
        let from = ParticipantId::new();
        let to = ParticipantId::new();
        svc.deposit(from, AssetSymbol::Usdc, dec!(100), None)
            .unwrap();
        svc.freeze(from, AssetSymbol::Usdc, dec!(50), None)
            .unwrap();
        svc.settle_transfer(from, to, AssetSymbol::Usdc, dec!(50), None)
            .unwrap();

        let from_bal = svc.get_balance(from, AssetSymbol::Usdc);
        assert_eq!(from_bal.available, dec!(50));
        assert_eq!(from_bal.frozen, dec!(0));

        let to_bal = svc.get_balance(to, AssetSymbol::Usdc);
        assert_eq!(to_bal.available, dec!(50));
    }

    #[test]
    fn test_ledger_entries() {
        let svc = WalletService::new();
        let pid = ParticipantId::new();
        svc.deposit(pid, AssetSymbol::Usdc, dec!(100), None)
            .unwrap();
        svc.freeze(pid, AssetSymbol::Usdc, dec!(30), None).unwrap();
        let entries = svc.get_ledger_entries(pid);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_quarantine_and_unquarantine() {
        let svc = WalletService::new();
        let pid = ParticipantId::new();
        svc.deposit(pid, AssetSymbol::Usdc, dec!(100), None)
            .unwrap();

        svc.quarantine(pid, AssetSymbol::Usdc, dec!(40), "AML alert".into())
            .unwrap();
        let bal = svc.get_balance(pid, AssetSymbol::Usdc);
        assert_eq!(bal.available, dec!(60));
        assert_eq!(bal.quarantined, dec!(40));
        assert!(bal.has_quarantined());

        // Cannot withdraw while quarantined
        let result = svc.withdraw(pid, AssetSymbol::Usdc, dec!(10), None);
        assert!(result.is_err());

        // Admin clears quarantine
        svc.unquarantine(pid, AssetSymbol::Usdc, dec!(40)).unwrap();
        let bal = svc.get_balance(pid, AssetSymbol::Usdc);
        assert_eq!(bal.available, dec!(100));
        assert_eq!(bal.quarantined, dec!(0));
    }

    #[test]
    fn test_slash_and_robin_hood_distribution() {
        let svc = WalletService::new();
        let malicious = ParticipantId::new();
        let honest_a = ParticipantId::new();
        let honest_b = ParticipantId::new();

        // Malicious agent has a frozen micro-stake
        svc.deposit(malicious, AssetSymbol::Usdc, dec!(1), None)
            .unwrap();
        svc.freeze(malicious, AssetSymbol::Usdc, dec!(0.1), None)
            .unwrap();

        // Slash and distribute to honest agents
        svc.slash_and_distribute(
            malicious,
            AssetSymbol::Usdc,
            dec!(0.1),
            &[honest_a, honest_b],
            None,
        )
        .unwrap();

        let mal_bal = svc.get_balance(malicious, AssetSymbol::Usdc);
        assert_eq!(mal_bal.frozen, dec!(0));

        let a_bal = svc.get_balance(honest_a, AssetSymbol::Usdc);
        let b_bal = svc.get_balance(honest_b, AssetSymbol::Usdc);
        assert_eq!(a_bal.available + b_bal.available, dec!(0.1));
    }

    #[test]
    fn test_slash_no_honest_agents_goes_to_faucet() {
        let svc = WalletService::new();
        let malicious = ParticipantId::new();

        svc.deposit(malicious, AssetSymbol::Usdc, dec!(1), None)
            .unwrap();
        svc.freeze(malicious, AssetSymbol::Usdc, dec!(0.1), None)
            .unwrap();

        svc.slash_and_distribute(malicious, AssetSymbol::Usdc, dec!(0.1), &[], None)
            .unwrap();

        let faucet_bal = svc.get_balance(svc.faucet_account, AssetSymbol::Usdc);
        assert_eq!(faucet_bal.available, dec!(0.1));
    }
}

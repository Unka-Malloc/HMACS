use crate::balance::Balance;
use crate::ledger::{LedgerEntry, LedgerEntryType};
use chrono::Utc;
use hmacs_core::{AssetSymbol, HmacsError, HmacsResult, ParticipantId, TransactionId, WalletId};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::collections::HashMap;

type BalanceKey = (ParticipantId, AssetSymbol);

/// In-memory wallet service for development. Production would use hmacs-storage.
pub struct WalletService {
    balances: RwLock<HashMap<BalanceKey, Balance>>,
    ledger: RwLock<Vec<LedgerEntry>>,
}

impl WalletService {
    pub fn new() -> Self {
        Self {
            balances: RwLock::new(HashMap::new()),
            ledger: RwLock::new(Vec::new()),
        }
    }

    pub fn get_balance(
        &self,
        participant_id: ParticipantId,
        asset: AssetSymbol,
    ) -> Balance {
        let key = (participant_id, asset);
        self.balances
            .read()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Balance {
                wallet_id: WalletId::new(),
                participant_id,
                asset,
                available: Decimal::ZERO,
                frozen: Decimal::ZERO,
                updated_at: Utc::now(),
            })
    }

    pub fn get_all_balances(&self, participant_id: ParticipantId) -> Vec<Balance> {
        self.balances
            .read()
            .iter()
            .filter(|((pid, _), _)| *pid == participant_id)
            .map(|(_, b): (&BalanceKey, &Balance)| b.clone())
            .collect()
    }

    pub fn deposit(
        &self,
        participant_id: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
        reference_id: Option<String>,
    ) -> HmacsResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(HmacsError::InvalidInput("Deposit amount must be positive".into()));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances.entry(key).or_insert_with(|| Balance {
            wallet_id: WalletId::new(),
            participant_id,
            asset,
            available: Decimal::ZERO,
            frozen: Decimal::ZERO,
            updated_at: Utc::now(),
        });

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
    ) -> HmacsResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(HmacsError::InvalidInput("Withdrawal amount must be positive".into()));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances
            .get_mut(&key)
            .ok_or_else(|| HmacsError::InsufficientBalance {
                needed: format!("{} {}", amount, asset),
                available: format!("0 {}", asset),
            })?;

        if !balance.can_spend(amount) {
            return Err(HmacsError::InsufficientBalance {
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
    ) -> HmacsResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(HmacsError::InvalidInput("Freeze amount must be positive".into()));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances
            .get_mut(&key)
            .ok_or_else(|| HmacsError::InsufficientBalance {
                needed: format!("{} {}", amount, asset),
                available: format!("0 {}", asset),
            })?;

        if !balance.can_spend(amount) {
            return Err(HmacsError::InsufficientBalance {
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
    ) -> HmacsResult<Balance> {
        if amount <= Decimal::ZERO {
            return Err(HmacsError::InvalidInput("Unfreeze amount must be positive".into()));
        }

        let key = (participant_id, asset);
        let mut balances = self.balances.write();
        let balance = balances
            .get_mut(&key)
            .ok_or_else(|| HmacsError::not_found("Balance", participant_id))?;

        if !balance.can_unfreeze(amount) {
            return Err(HmacsError::InvalidInput(format!(
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

    /// Transfer frozen funds from one participant to another (settlement).
    pub fn settle_transfer(
        &self,
        from: ParticipantId,
        to: ParticipantId,
        asset: AssetSymbol,
        amount: Decimal,
        reference_id: Option<String>,
    ) -> HmacsResult<()> {
        if amount <= Decimal::ZERO {
            return Err(HmacsError::InvalidInput("Transfer amount must be positive".into()));
        }

        let from_key = (from, asset);
        let to_key = (to, asset);

        let mut balances = self.balances.write();

        {
            let from_balance = balances
                .get_mut(&from_key)
                .ok_or_else(|| HmacsError::not_found("Balance", from))?;
            if !from_balance.can_unfreeze(amount) {
                return Err(HmacsError::InsufficientBalance {
                    needed: format!("{} {} (frozen)", amount, asset),
                    available: format!("{} {} (frozen)", from_balance.frozen, asset),
                });
            }
            from_balance.frozen -= amount;
            from_balance.updated_at = Utc::now();
        }

        let to_balance = balances.entry(to_key).or_insert_with(|| Balance {
            wallet_id: WalletId::new(),
            participant_id: to,
            asset,
            available: Decimal::ZERO,
            frozen: Decimal::ZERO,
            updated_at: Utc::now(),
        });
        to_balance.available += amount;
        to_balance.updated_at = Utc::now();

        let from_available = balances.get(&from_key).map(|b| b.available).unwrap_or_default();
        let to_available = balances.get(&to_key).map(|b| b.available).unwrap_or_default();

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
        svc.deposit(pid, AssetSymbol::Usdc, dec!(100), None).unwrap();
        let bal = svc.get_balance(pid, AssetSymbol::Usdc);
        assert_eq!(bal.available, dec!(100));
        assert_eq!(bal.frozen, dec!(0));
    }

    #[test]
    fn test_freeze_unfreeze() {
        let svc = WalletService::new();
        let pid = ParticipantId::new();
        svc.deposit(pid, AssetSymbol::Usdc, dec!(100), None).unwrap();
        svc.freeze(pid, AssetSymbol::Usdc, dec!(30), None).unwrap();
        let bal = svc.get_balance(pid, AssetSymbol::Usdc);
        assert_eq!(bal.available, dec!(70));
        assert_eq!(bal.frozen, dec!(30));

        svc.unfreeze(pid, AssetSymbol::Usdc, dec!(10), None).unwrap();
        let bal = svc.get_balance(pid, AssetSymbol::Usdc);
        assert_eq!(bal.available, dec!(80));
        assert_eq!(bal.frozen, dec!(20));
    }

    #[test]
    fn test_insufficient_balance() {
        let svc = WalletService::new();
        let pid = ParticipantId::new();
        svc.deposit(pid, AssetSymbol::Usdc, dec!(50), None).unwrap();
        let result = svc.withdraw(pid, AssetSymbol::Usdc, dec!(100), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_settle_transfer() {
        let svc = WalletService::new();
        let from = ParticipantId::new();
        let to = ParticipantId::new();
        svc.deposit(from, AssetSymbol::Usdc, dec!(100), None).unwrap();
        svc.freeze(from, AssetSymbol::Usdc, dec!(50), None).unwrap();
        svc.settle_transfer(from, to, AssetSymbol::Usdc, dec!(50), None).unwrap();

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
        svc.deposit(pid, AssetSymbol::Usdc, dec!(100), None).unwrap();
        svc.freeze(pid, AssetSymbol::Usdc, dec!(30), None).unwrap();
        let entries = svc.get_ledger_entries(pid);
        assert_eq!(entries.len(), 2);
    }
}

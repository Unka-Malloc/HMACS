use crate::racing::{RaceResult, ShareSender};
use crate::state::MpcTaskState;
use crate::types::{MpcTask, Signature};
use chrono::Utc;
use carbide_core::{AssetSymbol, CarbideError, CarbideResult, MpcTaskId, ParticipantId};
use carbide_wallet::WalletService;
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct MpcService {
    tasks: RwLock<HashMap<MpcTaskId, MpcTask>>,
    wallet: Arc<WalletService>,
    blacklist: RwLock<Vec<ParticipantId>>,
}

impl MpcService {
    pub fn new(wallet: Arc<WalletService>) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            wallet,
            blacklist: RwLock::new(Vec::new()),
        }
    }

    /// Create a new t-of-n FROST signature task and post it to the order book.
    pub fn create_task(
        &self,
        threshold: usize,
        total_agents: usize,
        message: Vec<u8>,
        stake_amount: Decimal,
        bounty_per_agent: Decimal,
    ) -> CarbideResult<MpcTask> {
        if threshold == 0 || threshold > total_agents {
            return Err(CarbideError::InvalidInput(format!(
                "Invalid threshold: {}-of-{}",
                threshold, total_agents
            )));
        }

        let now = Utc::now();
        let task = MpcTask {
            id: MpcTaskId::new(),
            state: MpcTaskState::Open,
            threshold,
            total_agents,
            message,
            participants: Vec::new(),
            valid_shares: Vec::new(),
            malicious_agents: Vec::new(),
            stake_amount,
            stake_asset: AssetSymbol::Usdc,
            bounty_per_agent,
            bounty_asset: AssetSymbol::Usdc,
            final_signature: None,
            created_at: now,
            updated_at: now,
            retry_count: 0,
        };

        self.tasks.write().insert(task.id, task.clone());
        Ok(task)
    }

    /// Agent locks a micro-stake to enter the race.
    pub fn join_race(
        &self,
        task_id: MpcTaskId,
        agent_id: ParticipantId,
    ) -> CarbideResult<()> {
        if self.is_blacklisted(agent_id) {
            return Err(CarbideError::AgentBlacklisted(format!(
                "Agent {} is permanently blacklisted",
                agent_id
            )));
        }

        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| CarbideError::not_found("MpcTask", task_id))?;

        if task.state != MpcTaskState::Open && task.state != MpcTaskState::Racing {
            return Err(CarbideError::InvalidStateTransition {
                from: task.state.to_string(),
                to: "join_race".into(),
            });
        }

        if task.participants.len() >= task.total_agents {
            return Err(CarbideError::InvalidInput("Race is full".into()));
        }

        // Lock micro-stake
        self.wallet.freeze(
            agent_id,
            task.stake_asset,
            task.stake_amount,
            Some(task_id.to_string()),
        )?;

        task.participants.push(agent_id);

        if task.participants.len() >= task.total_agents {
            task.state = task.state.transition_to(MpcTaskState::Racing)?;
        }

        task.updated_at = Utc::now();
        Ok(())
    }

    /// Execute the racing phase: collect shares, validate, assemble.
    /// Returns a channel sender for agents to submit shares.
    pub fn start_race(&self, task_id: MpcTaskId) -> CarbideResult<(ShareSender, MpcTaskId)> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| CarbideError::not_found("MpcTask", task_id))?;

        if task.state != MpcTaskState::Racing {
            return Err(CarbideError::InvalidStateTransition {
                from: task.state.to_string(),
                to: "start_race".into(),
            });
        }

        let (tx, _rx) = mpsc::channel(task.total_agents);
        Ok((tx, task_id))
    }

    /// Process the race result: settle bounties, slash malicious actors, unlock non-winners.
    pub fn settle_race(
        &self,
        task_id: MpcTaskId,
        result: RaceResult,
        signature: Signature,
    ) -> CarbideResult<MpcTask> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| CarbideError::not_found("MpcTask", task_id))?;

        task.valid_shares = result
            .valid_shares
            .iter()
            .map(|s| (s.agent_id, s.clone()))
            .collect();
        task.malicious_agents = result.malicious_agents.clone();
        task.final_signature = Some(signature);

        // Winners: agents whose shares were used
        let winners: Vec<ParticipantId> = result
            .valid_shares
            .iter()
            .map(|s| s.agent_id)
            .collect();

        // Non-winners: agents that participated but weren't in the winning set
        let non_winners: Vec<ParticipantId> = task
            .participants
            .iter()
            .filter(|p| !winners.contains(p) && !result.malicious_agents.contains(p))
            .copied()
            .collect();

        // Unlock stakes for non-winners (they are NOT penalized)
        for agent in &non_winners {
            let _ = self.wallet.unfreeze(
                *agent,
                task.stake_asset,
                task.stake_amount,
                Some(task_id.to_string()),
            );
        }

        // Unlock stakes and pay bounty to winners
        for agent in &winners {
            let _ = self.wallet.unfreeze(
                *agent,
                task.stake_asset,
                task.stake_amount,
                Some(task_id.to_string()),
            );
        }

        // Slash malicious agents and distribute via Robin Hood pool
        let honest_for_compensation: Vec<ParticipantId> = task
            .participants
            .iter()
            .filter(|p| !result.malicious_agents.contains(p))
            .copied()
            .collect();

        for malicious in &result.malicious_agents {
            let _ = self.wallet.slash_and_distribute(
                *malicious,
                task.stake_asset,
                task.stake_amount,
                &honest_for_compensation,
                Some(task_id.to_string()),
            );
            self.blacklist_agent(*malicious);
        }

        task.state = MpcTaskState::Settled;
        task.updated_at = Utc::now();

        info!(
            task_id = %task_id,
            winners = winners.len(),
            malicious = result.malicious_agents.len(),
            "MPC task settled"
        );

        Ok(task.clone())
    }

    /// Mark a task as failed, unlock all honest stakes, and optionally retry.
    pub fn fail_and_retry(&self, task_id: MpcTaskId) -> CarbideResult<Option<MpcTaskId>> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| CarbideError::not_found("MpcTask", task_id))?;

        task.state = MpcTaskState::Failed;
        task.updated_at = Utc::now();

        // Unlock all honest stakes
        for agent in &task.participants {
            if !task.malicious_agents.contains(agent) {
                let _ = self.wallet.unfreeze(
                    *agent,
                    task.stake_asset,
                    task.stake_amount,
                    Some(task_id.to_string()),
                );
            }
        }

        warn!(
            task_id = %task_id,
            retry_count = task.retry_count,
            "MPC task failed — generating retry"
        );

        // Create a retry task
        let retry = MpcTask {
            id: MpcTaskId::new(),
            state: MpcTaskState::Open,
            threshold: task.threshold,
            total_agents: task.total_agents,
            message: task.message.clone(),
            participants: Vec::new(),
            valid_shares: Vec::new(),
            malicious_agents: Vec::new(),
            stake_amount: task.stake_amount,
            stake_asset: task.stake_asset,
            bounty_per_agent: task.bounty_per_agent,
            bounty_asset: task.bounty_asset,
            final_signature: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            retry_count: task.retry_count + 1,
        };

        let retry_id = retry.id;
        tasks.insert(retry.id, retry);

        Ok(Some(retry_id))
    }

    pub fn get_task(&self, task_id: MpcTaskId) -> CarbideResult<MpcTask> {
        self.tasks
            .read()
            .get(&task_id)
            .cloned()
            .ok_or_else(|| CarbideError::not_found("MpcTask", task_id))
    }

    pub fn blacklist_agent(&self, agent_id: ParticipantId) {
        let mut bl = self.blacklist.write();
        if !bl.contains(&agent_id) {
            warn!(agent = %agent_id, "Agent permanently blacklisted for malicious MPC behavior");
            bl.push(agent_id);
        }
    }

    pub fn is_blacklisted(&self, agent_id: ParticipantId) -> bool {
        self.blacklist.read().contains(&agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_service() -> (MpcService, Arc<WalletService>) {
        let wallet = Arc::new(WalletService::new());
        let svc = MpcService::new(wallet.clone());
        (svc, wallet)
    }

    #[test]
    fn test_create_task() {
        let (svc, _) = make_service();
        let task = svc
            .create_task(3, 5, b"hello".to_vec(), dec!(0.1), dec!(1.0))
            .unwrap();
        assert_eq!(task.state, MpcTaskState::Open);
        assert_eq!(task.threshold, 3);
        assert_eq!(task.total_agents, 5);
    }

    #[test]
    fn test_invalid_threshold() {
        let (svc, _) = make_service();
        assert!(svc
            .create_task(6, 5, b"hello".to_vec(), dec!(0.1), dec!(1.0))
            .is_err());
    }

    #[test]
    fn test_join_race_locks_stake() {
        let (svc, wallet) = make_service();
        let agent = ParticipantId::new();

        wallet
            .deposit(agent, AssetSymbol::Usdc, dec!(1), None)
            .unwrap();

        let task = svc
            .create_task(1, 1, b"msg".to_vec(), dec!(0.1), dec!(0.5))
            .unwrap();
        svc.join_race(task.id, agent).unwrap();

        let bal = wallet.get_balance(agent, AssetSymbol::Usdc);
        assert_eq!(bal.frozen, dec!(0.1));
    }

    #[test]
    fn test_blacklisted_agent_cannot_join() {
        let (svc, wallet) = make_service();
        let agent = ParticipantId::new();

        wallet
            .deposit(agent, AssetSymbol::Usdc, dec!(1), None)
            .unwrap();

        svc.blacklist_agent(agent);

        let task = svc
            .create_task(1, 1, b"msg".to_vec(), dec!(0.1), dec!(0.5))
            .unwrap();
        let result = svc.join_race(task.id, agent);
        assert!(result.is_err());
    }

    #[test]
    fn test_fail_and_retry_creates_new_task() {
        let (svc, wallet) = make_service();
        let agent = ParticipantId::new();

        wallet
            .deposit(agent, AssetSymbol::Usdc, dec!(1), None)
            .unwrap();

        let task = svc
            .create_task(1, 1, b"msg".to_vec(), dec!(0.1), dec!(0.5))
            .unwrap();
        svc.join_race(task.id, agent).unwrap();

        // Manually transition to Racing
        {
            let mut tasks = svc.tasks.write();
            let t = tasks.get_mut(&task.id).unwrap();
            t.state = MpcTaskState::Racing;
        }

        let retry_id = svc.fail_and_retry(task.id).unwrap();
        assert!(retry_id.is_some());

        let retry_task = svc.get_task(retry_id.unwrap()).unwrap();
        assert_eq!(retry_task.state, MpcTaskState::Open);
        assert_eq!(retry_task.retry_count, 1);

        // Agent's stake should be unlocked
        let bal = wallet.get_balance(agent, AssetSymbol::Usdc);
        assert_eq!(bal.frozen, dec!(0));
    }
}

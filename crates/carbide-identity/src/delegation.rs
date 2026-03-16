use chrono::{Duration, Utc};
use carbide_core::{
    AgentPermission, DelegatedKey, DelegatedKeyId, CarbideError, CarbideResult, ParticipantId,
    ParticipantKind,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request to create a delegated session key for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDelegatedKeyRequest {
    pub agent_id: ParticipantId,
    pub public_key: String,
    pub permissions: Vec<AgentPermission>,
    pub ttl_hours: Option<i64>,
}

/// Manages delegated keys that allow agents to act on behalf of humans.
pub struct DelegationService {
    keys: RwLock<HashMap<DelegatedKeyId, DelegatedKey>>,
    agent_keys: RwLock<HashMap<ParticipantId, Vec<DelegatedKeyId>>>,
}

impl DelegationService {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
            agent_keys: RwLock::new(HashMap::new()),
        }
    }

    /// A Human master issues a delegated key to an Agent.
    pub fn create_delegated_key(
        &self,
        master_id: ParticipantId,
        master_kind: ParticipantKind,
        request: CreateDelegatedKeyRequest,
    ) -> CarbideResult<DelegatedKey> {
        if master_kind != ParticipantKind::Human {
            return Err(CarbideError::Forbidden(
                "Only Human participants can issue delegated keys".into(),
            ));
        }

        let ttl = request.ttl_hours.unwrap_or(24);
        let now = Utc::now();
        let key = DelegatedKey {
            id: DelegatedKeyId::new(),
            agent_id: request.agent_id,
            master_id,
            public_key: request.public_key,
            permissions: request.permissions,
            created_at: now,
            expires_at: now + Duration::hours(ttl),
            revoked: false,
        };

        self.keys.write().insert(key.id, key.clone());
        self.agent_keys
            .write()
            .entry(request.agent_id)
            .or_default()
            .push(key.id);

        Ok(key)
    }

    pub fn get_key(&self, key_id: DelegatedKeyId) -> CarbideResult<DelegatedKey> {
        self.keys
            .read()
            .get(&key_id)
            .cloned()
            .ok_or_else(|| CarbideError::not_found("DelegatedKey", key_id))
    }

    pub fn revoke_key(&self, key_id: DelegatedKeyId, master_id: ParticipantId) -> CarbideResult<()> {
        let mut keys = self.keys.write();
        let key = keys
            .get_mut(&key_id)
            .ok_or_else(|| CarbideError::not_found("DelegatedKey", key_id))?;

        if key.master_id != master_id {
            return Err(CarbideError::Forbidden(
                "Only the issuing master can revoke a delegated key".into(),
            ));
        }

        key.revoked = true;
        Ok(())
    }

    /// Verify that an agent has a valid delegated key with the required permission.
    pub fn check_agent_permission(
        &self,
        agent_id: ParticipantId,
        permission: AgentPermission,
    ) -> CarbideResult<DelegatedKey> {
        let agent_keys = self.agent_keys.read();
        let key_ids = agent_keys
            .get(&agent_id)
            .ok_or_else(|| CarbideError::Unauthorized("No delegated keys for this agent".into()))?;

        let keys = self.keys.read();
        for kid in key_ids.iter().rev() {
            if let Some(key) = keys.get(kid) {
                if key.has_permission(permission) {
                    return Ok(key.clone());
                }
            }
        }

        Err(CarbideError::Forbidden(format!(
            "Agent {} does not have '{}' permission",
            agent_id, permission
        )))
    }

    /// Hard rejection: agents cannot call fund-transfer functions.
    pub fn enforce_no_fund_transfer(
        &self,
        participant_kind: ParticipantKind,
    ) -> CarbideResult<()> {
        if participant_kind == ParticipantKind::Agent {
            return Err(CarbideError::AgentFundTransferForbidden);
        }
        Ok(())
    }

    pub fn get_active_keys_for_agent(&self, agent_id: ParticipantId) -> Vec<DelegatedKey> {
        let agent_keys = self.agent_keys.read();
        let Some(key_ids) = agent_keys.get(&agent_id) else {
            return vec![];
        };

        let keys = self.keys.read();
        key_ids
            .iter()
            .filter_map(|kid| keys.get(kid))
            .filter(|k| k.is_valid())
            .cloned()
            .collect()
    }
}

impl Default for DelegationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_check_permission() {
        let svc = DelegationService::new();
        let master = ParticipantId::new();
        let agent = ParticipantId::new();

        svc.create_delegated_key(
            master,
            ParticipantKind::Human,
            CreateDelegatedKeyRequest {
                agent_id: agent,
                public_key: "test_pubkey".into(),
                permissions: vec![AgentPermission::AcceptTask, AgentPermission::SubmitResult],
                ttl_hours: Some(1),
            },
        )
        .unwrap();

        assert!(svc
            .check_agent_permission(agent, AgentPermission::AcceptTask)
            .is_ok());
        assert!(svc
            .check_agent_permission(agent, AgentPermission::SubmitResult)
            .is_ok());
    }

    #[test]
    fn test_agent_cannot_issue_keys() {
        let svc = DelegationService::new();
        let result = svc.create_delegated_key(
            ParticipantId::new(),
            ParticipantKind::Agent,
            CreateDelegatedKeyRequest {
                agent_id: ParticipantId::new(),
                public_key: "key".into(),
                permissions: vec![AgentPermission::AcceptTask],
                ttl_hours: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_revoke_key() {
        let svc = DelegationService::new();
        let master = ParticipantId::new();
        let agent = ParticipantId::new();

        let key = svc
            .create_delegated_key(
                master,
                ParticipantKind::Human,
                CreateDelegatedKeyRequest {
                    agent_id: agent,
                    public_key: "key".into(),
                    permissions: vec![AgentPermission::AcceptTask],
                    ttl_hours: None,
                },
            )
            .unwrap();

        svc.revoke_key(key.id, master).unwrap();

        assert!(svc
            .check_agent_permission(agent, AgentPermission::AcceptTask)
            .is_err());
    }

    #[test]
    fn test_enforce_no_fund_transfer_agent() {
        let svc = DelegationService::new();
        assert!(svc
            .enforce_no_fund_transfer(ParticipantKind::Agent)
            .is_err());
        assert!(svc
            .enforce_no_fund_transfer(ParticipantKind::Human)
            .is_ok());
    }
}

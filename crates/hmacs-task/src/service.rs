use crate::bid::{Bid, BidStatus, CreateBid};
use crate::model::{CreateTask, DeliverTask, Task};
use crate::state_machine::TaskStatus;
use chrono::Utc;
use hmacs_core::{BidId, HmacsError, HmacsResult, ParticipantId, TaskId};
use parking_lot::RwLock;
use std::collections::HashMap;

pub struct TaskService {
    tasks: RwLock<HashMap<TaskId, Task>>,
    bids: RwLock<HashMap<BidId, Bid>>,
}

impl TaskService {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            bids: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_task(
        &self,
        creator_id: ParticipantId,
        input: CreateTask,
    ) -> HmacsResult<Task> {
        let now = Utc::now();
        let task = Task {
            id: TaskId::new(),
            creator_id,
            title: input.title,
            description: input.description,
            skill_tags: input.skill_tags,
            budget_asset: input.budget_asset,
            budget_amount: input.budget_amount,
            status: TaskStatus::Open,
            restriction: input.restriction,
            assigned_to: None,
            deadline: input.deadline,
            created_at: now,
            updated_at: now,
            completed_at: None,
            deliverable_url: None,
            deliverable_notes: None,
        };
        self.tasks.write().insert(task.id, task.clone());
        Ok(task)
    }

    pub fn get_task(&self, task_id: TaskId) -> HmacsResult<Task> {
        self.tasks
            .read()
            .get(&task_id)
            .cloned()
            .ok_or_else(|| HmacsError::not_found("Task", task_id))
    }

    pub fn list_tasks(&self, status: Option<TaskStatus>) -> Vec<Task> {
        self.tasks
            .read()
            .values()
            .filter(|t| status.is_none_or(|s| t.status == s))
            .cloned()
            .collect()
    }

    pub fn place_bid(
        &self,
        bidder_id: ParticipantId,
        input: CreateBid,
    ) -> HmacsResult<Bid> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&input.task_id)
            .ok_or_else(|| HmacsError::not_found("Task", input.task_id))?;

        if task.status != TaskStatus::Open && task.status != TaskStatus::Bidding {
            return Err(HmacsError::InvalidInput(
                "Task is not accepting bids".into(),
            ));
        }

        if task.status == TaskStatus::Open {
            task.status = TaskStatus::Bidding;
            task.updated_at = Utc::now();
        }

        let now = Utc::now();
        let bid = Bid {
            id: BidId::new(),
            task_id: input.task_id,
            bidder_id,
            amount: input.amount,
            asset: input.asset,
            proposal: input.proposal,
            estimated_duration_hours: input.estimated_duration_hours,
            status: BidStatus::Pending,
            created_at: now,
            updated_at: now,
        };

        self.bids.write().insert(bid.id, bid.clone());
        Ok(bid)
    }

    pub fn accept_bid(
        &self,
        task_id: TaskId,
        bid_id: BidId,
        caller_id: ParticipantId,
    ) -> HmacsResult<(Task, Bid)> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| HmacsError::not_found("Task", task_id))?;

        if task.creator_id != caller_id {
            return Err(HmacsError::Forbidden(
                "Only the task creator can accept bids".into(),
            ));
        }

        task.status = task.status.transition_to(TaskStatus::Assigned)?;

        let mut bids = self.bids.write();

        // Validate the bid exists and belongs to this task
        {
            let bid = bids
                .get(&bid_id)
                .ok_or_else(|| HmacsError::not_found("Bid", bid_id))?;
            if bid.task_id != task_id {
                return Err(HmacsError::InvalidInput("Bid does not belong to this task".into()));
            }
            task.assigned_to = Some(bid.bidder_id);
            task.updated_at = Utc::now();
        }

        // Collect all bid IDs for this task to process
        let all_bid_ids: Vec<BidId> = bids
            .values()
            .filter(|b| b.task_id == task_id)
            .map(|b| b.id)
            .collect();

        let now = Utc::now();
        for id in &all_bid_ids {
            if let Some(b) = bids.get_mut(id) {
                if b.id == bid_id {
                    b.status = BidStatus::Accepted;
                } else if b.status == BidStatus::Pending {
                    b.status = BidStatus::Rejected;
                }
                b.updated_at = now;
            }
        }

        let bid = bids.get(&bid_id).cloned()
            .ok_or_else(|| HmacsError::not_found("Bid", bid_id))?;

        Ok((task.clone(), bid))
    }

    pub fn start_task(
        &self,
        task_id: TaskId,
        worker_id: ParticipantId,
    ) -> HmacsResult<Task> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| HmacsError::not_found("Task", task_id))?;

        if task.assigned_to != Some(worker_id) {
            return Err(HmacsError::Forbidden(
                "Only the assigned worker can start the task".into(),
            ));
        }

        task.status = task.status.transition_to(TaskStatus::InProgress)?;
        task.updated_at = Utc::now();
        Ok(task.clone())
    }

    pub fn deliver_task(
        &self,
        task_id: TaskId,
        worker_id: ParticipantId,
        delivery: DeliverTask,
    ) -> HmacsResult<Task> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| HmacsError::not_found("Task", task_id))?;

        if task.assigned_to != Some(worker_id) {
            return Err(HmacsError::Forbidden(
                "Only the assigned worker can deliver".into(),
            ));
        }

        task.status = task.status.transition_to(TaskStatus::Delivered)?;
        task.deliverable_url = delivery.deliverable_url;
        task.deliverable_notes = delivery.deliverable_notes;
        task.updated_at = Utc::now();
        Ok(task.clone())
    }

    pub fn approve_delivery(
        &self,
        task_id: TaskId,
        caller_id: ParticipantId,
    ) -> HmacsResult<Task> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| HmacsError::not_found("Task", task_id))?;

        if task.creator_id != caller_id {
            return Err(HmacsError::Forbidden(
                "Only the task creator can approve delivery".into(),
            ));
        }

        task.status = task.status.transition_to(TaskStatus::Completed)?;
        task.completed_at = Some(Utc::now());
        task.updated_at = Utc::now();
        Ok(task.clone())
    }

    pub fn dispute_delivery(
        &self,
        task_id: TaskId,
        caller_id: ParticipantId,
    ) -> HmacsResult<Task> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| HmacsError::not_found("Task", task_id))?;

        if task.creator_id != caller_id {
            return Err(HmacsError::Forbidden(
                "Only the task creator can dispute".into(),
            ));
        }

        task.status = task.status.transition_to(TaskStatus::Disputed)?;
        task.updated_at = Utc::now();
        Ok(task.clone())
    }

    pub fn cancel_task(
        &self,
        task_id: TaskId,
        caller_id: ParticipantId,
    ) -> HmacsResult<Task> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| HmacsError::not_found("Task", task_id))?;

        if task.creator_id != caller_id {
            return Err(HmacsError::Forbidden(
                "Only the task creator can cancel".into(),
            ));
        }

        task.status = task.status.transition_to(TaskStatus::Cancelled)?;
        task.updated_at = Utc::now();
        Ok(task.clone())
    }

    pub fn get_bids_for_task(&self, task_id: TaskId) -> Vec<Bid> {
        self.bids
            .read()
            .values()
            .filter(|b| b.task_id == task_id)
            .cloned()
            .collect()
    }
}

impl Default for TaskService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmacs_core::{AssetSymbol, ParticipantRestriction};
    use rust_decimal_macros::dec;

    fn sample_create_task() -> CreateTask {
        CreateTask {
            title: "Build a website".into(),
            description: "Need a landing page".into(),
            skill_tags: vec!["web".into(), "design".into()],
            budget_asset: AssetSymbol::Usdc,
            budget_amount: dec!(500),
            restriction: ParticipantRestriction::Any,
            deadline: None,
        }
    }

    #[test]
    fn test_full_task_lifecycle() {
        let svc = TaskService::new();
        let creator = ParticipantId::new();
        let worker = ParticipantId::new();

        let task = svc.create_task(creator, sample_create_task()).unwrap();
        assert_eq!(task.status, TaskStatus::Open);

        let bid = svc
            .place_bid(
                worker,
                CreateBid {
                    task_id: task.id,
                    amount: dec!(450),
                    asset: AssetSymbol::Usdc,
                    proposal: "I can do it".into(),
                    estimated_duration_hours: Some(10.0),
                },
            )
            .unwrap();

        let (task, _) = svc.accept_bid(task.id, bid.id, creator).unwrap();
        assert_eq!(task.status, TaskStatus::Assigned);

        let task = svc.start_task(task.id, worker).unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);

        let task = svc
            .deliver_task(
                task.id,
                worker,
                DeliverTask {
                    deliverable_url: Some("https://example.com".into()),
                    deliverable_notes: Some("Done!".into()),
                },
            )
            .unwrap();
        assert_eq!(task.status, TaskStatus::Delivered);

        let task = svc.approve_delivery(task.id, creator).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
    }
}

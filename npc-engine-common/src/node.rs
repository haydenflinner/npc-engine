use std::{sync::{Arc, Weak}, collections::BTreeMap, fmt, mem, hash::{Hash, Hasher}};

use crate::{Domain, AgentId, Task, StateDiffRef, AgentValue};

/// Strong atomic reference counted node
pub type Node<D> = Arc<NodeInner<D>>;

/// Weak atomic reference counted node
pub type WeakNode<D> = Weak<NodeInner<D>>;

// FIXME: unpub
pub struct NodeInner<D: Domain> {
    pub diff: D::Diff,
    pub active_agent: AgentId,
    pub tasks: BTreeMap<AgentId, Box<dyn Task<D>>>,
    current_values: BTreeMap<AgentId, AgentValue>, // cached current values
}

impl<D: Domain> fmt::Debug for NodeInner<D> {
    fn fmt(&self, f: &'_ mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("NodeInner")
            .field("diff", &self.diff)
            .field("agent", &self.active_agent)
            .field("tasks", &"...")
            .field("current_values", &self.current_values)
            .finish()
    }
}

impl<D: Domain> NodeInner<D> {
    pub fn new(
        initial_state: &D::State,
        diff: D::Diff,
        active_agent: AgentId,
        mut tasks: BTreeMap<AgentId, Box<dyn Task<D>>>,
    ) -> Self {
        // Check validity of task for agent
        if let Some(task) = tasks.get(&active_agent) {
            if !task.is_valid(StateDiffRef::new(initial_state, &diff), active_agent) {
                tasks.remove(&active_agent);
            }
        }

        // FIXME: extract agent list from tasks and active_agent
        // Get observable agents
        let agents = D::get_visible_agents(
            StateDiffRef::new(initial_state, &diff),
            active_agent
        );
        debug_assert!(agents.contains(&active_agent));
        // Set child current values
        let current_values = agents
            .iter()
            .map(|agent| {
                (
                    *agent,
                    D::get_current_value(StateDiffRef::new(initial_state, &diff), *agent),
                )
            })
            .collect();


        NodeInner {
            active_agent,
            diff,
            tasks,
            current_values
        }
    }

    /// Returns agent who owns the node.
    pub fn agent(&self) -> AgentId {
        self.active_agent
    }

    /// Returns diff of current node.
    pub fn diff(&self) -> &D::Diff {
        &self.diff
    }

    /// Build a state-diff reference
    pub fn state_diff_ref<'a>(&'a self, initial_state: &'a D::State) -> StateDiffRef<'a, D> {
        StateDiffRef::new(
            initial_state,
            &self.diff,
        )
    }

    /// Return the current value from an agent, panic if not present in the node
    pub fn current_value(&self, agent: AgentId) -> AgentValue {
        *self.current_values.get(&agent).unwrap()
    }

    /// Return the current value from an agent, compute if not present in the node
    pub fn current_value_or_compute(&self, agent: AgentId, initial_state: &D::State) -> AgentValue {
        self.current_values
            .get(&agent)
            .copied()
            .unwrap_or_else(||
                D::get_current_value(
                    StateDiffRef::new(
                        initial_state,
                        &self.diff,
                    ),
                    agent,
                )
            )
    }

    /// Return all current values
    pub fn current_values(&self) -> &BTreeMap<AgentId, AgentValue> {
        &self.current_values
    }

    // Returns the size in bytes
    pub fn size(&self, task_size: fn(&dyn Task<D>) -> usize) -> usize {
        let mut size = 0;

        size += mem::size_of::<Self>();
        size += self.current_values.len() * mem::size_of::<(AgentId, f32)>();

        for task in self.tasks.values() {
            size += task_size(&**task);
        }

        size
    }
}

impl<D: Domain> Hash for NodeInner<D> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.active_agent.hash(hasher);
        self.diff.hash(hasher);
        self.tasks.hash(hasher);
    }
}

impl<D: Domain> PartialEq for NodeInner<D> {
    fn eq(&self, other: &Self) -> bool {
        self.active_agent.eq(&other.active_agent) && self.diff.eq(&other.diff) && self.tasks.eq(&other.tasks)
    }
}

impl<D: Domain> Eq for NodeInner<D> {}

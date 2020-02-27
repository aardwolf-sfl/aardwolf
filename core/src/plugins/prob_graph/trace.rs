use std::collections::{BTreeSet, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::iter::Peekable;

use petgraph::graph::{DefaultIx, DiGraph, NodeIndex};
use petgraph::Direction;

use super::models::{Model, Node, NodeType};
use super::pdg::create_pdg;
use crate::api::Api;
use crate::raw::data::Statement;

// We need BTreeSet because we want to keep a unique order of the elements
// for NodeState's implementation of Hash trait.
type DataContext<'a> = BTreeSet<(u64, &'a Statement)>;

pub trait DataContextExt<'a> {
    fn diff(&self, other: &Self) -> Vec<(u64, &'a Statement, &'a Statement)>;
}

impl<'a> DataContextExt<'a> for DataContext<'a> {
    fn diff(&self, other: &Self) -> Vec<(u64, &'a Statement, &'a Statement)> {
        let mut result = Vec::new();
        for (var, self_def) in self {
            if let Some((_, other_def)) = other
                .iter()
                .find(|(item_var, other_def)| var == item_var && self_def != other_def)
            {
                result.push((*var, *self_def, *other_def));
            }
        }
        result
    }
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone)]
pub enum NodeState<'a> {
    // Immediate successor on the path where the program flow went from the branching.
    Predicate(&'a Statement),
    // Variable and statement that defined the variable last.
    Data(DataContext<'a>),
    // When node has not been executed yet.
    NotExecuted,
    Executed,
}

impl<'a> NodeState<'a> {
    pub fn canonicalize(self) -> NodeState<'a> {
        match self {
            NodeState::Data(ctx) => {
                if ctx.is_empty() {
                    NodeState::Executed
                } else {
                    NodeState::Data(ctx)
                }
            }
            state => state,
        }
    }
}

impl<'a> Hash for NodeState<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            NodeState::Predicate(stmt) => stmt.hash(state),
            NodeState::Data(context) => {
                // Context is most likely very small set,
                // so the hashing operation is constant-time
                // (at least in majority of cases).
                for item in context.iter() {
                    item.hash(state);
                }
            }
            NodeState::NotExecuted => (0).hash(state),
            NodeState::Executed => (1).hash(state),
        }
    }
}

pub type StateConf<'a> = BTreeSet<(Node<'a>, NodeState<'a>)>;

pub trait StateConfExt<'a> {
    fn canonicalize(self) -> Option<Self>
    where
        Self: Sized;

    fn from_node(node: Node<'a>, state: NodeState<'a>) -> Self;
}

impl<'a> StateConfExt<'a> for StateConf<'a> {
    fn canonicalize(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }

    fn from_node(node: Node<'a>, state: NodeState<'a>) -> Self {
        [(node, state)].iter().cloned().collect()
    }
}

struct StackFrame<'a, N: Hash + Eq> {
    pub current_states: HashMap<N, NodeState<'a>>,
    pub current_defs: HashMap<u64, &'a Statement>,
}

impl<'a, N: Hash + Eq + Copy> StackFrame<'a, N> {
    pub fn new() -> Self {
        StackFrame {
            current_states: HashMap::new(),
            current_defs: HashMap::new(),
        }
    }

    pub fn update_state(&mut self, node: N, state: NodeState<'a>) {
        self.current_states.insert(node, state.canonicalize());
    }

    pub fn get_state(&self, node: &N) -> NodeState<'a> {
        self.current_states
            .get(node)
            .cloned()
            .unwrap_or(NodeState::NotExecuted)
    }

    pub fn get_data_state(&self, stmt: &'a Statement) -> NodeState<'a> {
        let mut state = BTreeSet::new();

        for var in stmt
            .uses
            .iter()
            .flat_map(|access| access.get_scalars_for_use())
        {
            if let Some(def) = self.current_defs.get(&var) {
                state.insert((var, *def));
            }
        }

        NodeState::Data(state)
    }

    pub fn update_defs(&mut self, stmt: &'a Statement) {
        // TODO: A data structure that tries to model data dependencies of pointers should be used.
        //       At least on a level, when a pointer is sent to a function and the function modifies it (or its child),
        //       then it should be added as a definition of the function call.
        for var in stmt
            .defs
            .iter()
            .flat_map(|access| access.get_scalars_for_def())
        {
            self.current_defs.insert(var, stmt);
        }
    }
}

pub struct TraceItem<'a> {
    pub node: Node<'a>,
    pub node_state: NodeState<'a>,
    pub parents_state_conf: Option<StateConf<'a>>,
}

impl<'a> TraceItem<'a> {
    pub fn new(
        node: Node<'a>,
        node_state: NodeState<'a>,
        parents_state_conf: Option<StateConf<'a>>,
    ) -> Self {
        TraceItem {
            node,
            node_state,
            parents_state_conf,
        }
    }
}

pub struct Trace<'a, I: Iterator<Item = &'a Statement>, M: Model<'a>> {
    stack_frames: Vec<StackFrame<'a, NodeIndex<DefaultIx>>>,
    trace: Peekable<I>,
    next_items: VecDeque<TraceItem<'a>>,
    api: &'a Api<'a>,
    models: HashMap<&'a String, M>,
}

impl<'a, I: Iterator<Item = &'a Statement>, M: Model<'a>> Trace<'a, I, M> {
    pub fn new(trace: I, api: &'a Api<'a>) -> Self {
        Trace {
            stack_frames: vec![StackFrame::new()],
            trace: trace.peekable(),
            next_items: VecDeque::with_capacity(2),
            api,
            models: HashMap::new(),
        }
    }
}

impl<'a, I: Iterator<Item = &'a Statement>, M: Model<'a>> Iterator for Trace<'a, I, M> {
    type Item = TraceItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.next_items.is_empty() {
            return self.next_items.pop_front();
        }

        // FIXME: Everywhere we clone values from or to self.parent,
        //        we could probably just take the value of leaving the option empty?

        let stmt = self.trace.next()?;
        let func = self.api.get_stmts().find_fn(stmt).unwrap();

        // There should always exist a stack frame. If there is not, then one of the following happened:
        //   * The function from top-level stack frame returned
        //     and there exists a statement in the trace that follows it.
        //   * We missed a function call and a return statement discarded wrong stack frame.
        let stack_frame = self.stack_frames.last_mut().unwrap();

        let cfgs = self.api.get_cfgs();

        // Get (or initialize) model for the function which the statement comes from.
        let model = self
            .models
            .entry(func)
            .or_insert_with(|| M::from_pdg(&create_pdg(cfgs.get(func).unwrap())))
            .get_graph();

        // Get all nodes from the model corresponding to the statement.
        let mut nodes = model
            .node_indices()
            .filter(|index| model[*index].stmt == stmt)
            .collect::<Vec<_>>();

        // Sort the nodes by type. The ordering of the type should ensure that nodes are sorted "topologically".
        nodes.sort_unstable_by_key(|index| model[*index].typ);

        for index in nodes {
            let node = model[index];

            let state = match node.typ {
                NodeType::Predicate => {
                    // Predicate node must have a successor, so we can unwrap.
                    let next = self.trace.peek().unwrap();
                    let state = NodeState::Predicate(next);

                    stack_frame.update_state(index, state.clone());

                    state
                }
                NodeType::NonPredicate => {
                    let state = stack_frame.get_data_state(stmt).canonicalize();

                    stack_frame.update_state(index, state.clone());
                    stack_frame.update_defs(stmt);

                    state
                }
                NodeType::SelfLoop => {
                    let state = NodeState::Executed;
                    stack_frame.update_state(index, state.clone());
                    state
                }
            };

            let mut parents = model
                .neighbors_directed(index, Direction::Incoming)
                .map(|parent| (model[parent], stack_frame.get_state(&parent)))
                .collect::<StateConf<'a>>();

            self.next_items
                .push_back(TraceItem::new(node, state, parents.canonicalize()));
        }

        if stmt.is_ret() {
            // This statement returns from a function,
            // hence we can throw associated stack frame away.
            self.stack_frames.pop();
        }

        // FIXME: This will not work with things like operator overloading, when a custom function is called
        //        but it cannot be determined using static analysis in the frontend for some reason
        //        (especially true for dynamic languages). But we should be able to determine a call
        //        by looking at the next statement - the statement is a call if the next one is not the statement's successor.
        if stmt.is_call() {
            // Initialize new stack frame which will be used in the called function.
            self.stack_frames.push(StackFrame::new());
        }

        self.next_items.pop_front()
    }
}

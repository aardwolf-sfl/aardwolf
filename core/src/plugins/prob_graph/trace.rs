use std::collections::{BTreeSet, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::iter::Peekable;

use petgraph::graph::{DefaultIx, NodeIndex};
use petgraph::Direction;

use super::models::{Model, Node, NodeType};
use crate::api::Api;
use crate::data::{access::AccessChain, statement::Statement, types::FuncName};

// We need BTreeSet because we want to keep a unique order of the elements
// for NodeState's implementation of Hash trait.
type DataContext<'data> = BTreeSet<(u64, &'data Statement)>;

pub trait DataContextExt<'data> {
    fn diff(&self, other: &Self) -> Vec<(u64, &'data Statement, &'data Statement)>;
}

impl<'data> DataContextExt<'data> for DataContext<'data> {
    fn diff(&self, other: &Self) -> Vec<(u64, &'data Statement, &'data Statement)> {
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
pub enum NodeState<'data> {
    // Immediate successor on the path where the program flow went from the branching.
    Predicate(&'data Statement),
    // Variable and statement that defined the variable last.
    Data(DataContext<'data>),
    // When node has not been executed yet.
    NotExecuted,
    Executed,
}

impl<'data> NodeState<'data> {
    pub fn canonicalize(self) -> NodeState<'data> {
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

impl<'data> Hash for NodeState<'data> {
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

pub type StateConf<'data> = BTreeSet<(Node<'data>, NodeState<'data>)>;

pub trait StateConfExt<'data> {
    fn canonicalize(self) -> Option<Self>
    where
        Self: Sized;

    fn from_node(node: Node<'data>, state: NodeState<'data>) -> Self;
}

impl<'data> StateConfExt<'data> for StateConf<'data> {
    fn canonicalize(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }

    fn from_node(node: Node<'data>, state: NodeState<'data>) -> Self {
        [(node, state)].iter().cloned().collect()
    }
}

struct StackFrame<'data, N: Hash + Eq> {
    pub current_states: HashMap<N, NodeState<'data>>,
    pub current_defs: HashMap<u64, &'data Statement>,
}

impl<'data, N: Hash + Eq + Copy> StackFrame<'data, N> {
    pub fn new() -> Self {
        StackFrame {
            current_states: HashMap::new(),
            current_defs: HashMap::new(),
        }
    }

    pub fn update_state(&mut self, node: N, state: NodeState<'data>) {
        self.current_states.insert(node, state.canonicalize());
    }

    pub fn get_state(&self, node: &N) -> NodeState<'data> {
        self.current_states
            .get(node)
            .cloned()
            .unwrap_or(NodeState::NotExecuted)
    }

    pub fn get_data_state(&self, stmt: &'data Statement) -> NodeState<'data> {
        let mut state = BTreeSet::new();

        for var in stmt.uses.iter().flat_map(AccessChain::from_uses) {
            if let Some(def) = self.current_defs.get(&var) {
                state.insert((var, *def));
            }
        }

        NodeState::Data(state)
    }

    pub fn update_defs(&mut self, stmt: &'data Statement) {
        // TODO: A data structure that tries to model data dependencies of pointers should be used.
        //       At least on a level, when a pointer is sent to a function and the function modifies it (or its child),
        //       then it should be added as a definition of the function call.
        for var in stmt.defs.iter().flat_map(AccessChain::from_defs) {
            self.current_defs.insert(var, stmt);
        }
    }
}

pub struct TraceItem<'data> {
    pub node: Node<'data>,
    pub node_state: NodeState<'data>,
    pub parents_state_conf: Option<StateConf<'data>>,
}

impl<'data> TraceItem<'data> {
    pub fn new(
        node: Node<'data>,
        node_state: NodeState<'data>,
        parents_state_conf: Option<StateConf<'data>>,
    ) -> Self {
        TraceItem {
            node,
            node_state,
            parents_state_conf,
        }
    }
}

pub struct Trace<'data, I: Iterator<Item = &'data Statement>, M: Model<'data>> {
    stack_frames: Vec<StackFrame<'data, NodeIndex<DefaultIx>>>,
    trace: Peekable<I>,
    next_items: VecDeque<TraceItem<'data>>,
    api: &'data Api<'data>,
    models: HashMap<FuncName, M>,
}

impl<'data, I: Iterator<Item = &'data Statement>, M: Model<'data>> Trace<'data, I, M> {
    pub fn new(trace: I, api: &'data Api<'data>) -> Self {
        Trace {
            stack_frames: vec![StackFrame::new()],
            trace: trace.peekable(),
            next_items: VecDeque::with_capacity(2),
            api,
            models: HashMap::new(),
        }
    }
}

impl<'data, I: Iterator<Item = &'data Statement>, M: Model<'data>> Iterator for Trace<'data, I, M> {
    type Item = TraceItem<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.next_items.is_empty() {
            return self.next_items.pop_front();
        }

        let stmt = self.trace.next()?;
        let func = self.api.get_stmts().find_fn(stmt).unwrap();

        // There should always exist a stack frame. If there is not, then one of the following happened:
        //   * The function from top-level stack frame returned
        //     and there exists a statement in the trace that follows it.
        //   * We missed a function call and a return statement discarded wrong stack frame.
        let stack_frame = self.stack_frames.last_mut().unwrap();

        let pdgs = self.api.get_pdgs();

        // Get (or initialize) model for the function which the statement comes from.
        let model = self
            .models
            .entry(func.clone())
            .or_insert_with(|| M::from_pdg(&pdgs.get(func).unwrap()))
            .get_graph();

        // Get all nodes from the model corresponding to the statement.
        for index in model[stmt].iter() {
            let node = model[*index];

            let state = match node.typ {
                NodeType::Predicate => {
                    // Predicate node must have a successor, so we can unwrap.
                    let next = self.trace.peek().unwrap();
                    let state = NodeState::Predicate(next);

                    stack_frame.update_state(*index, state.clone());

                    state
                }
                NodeType::NonPredicate => {
                    let state = stack_frame.get_data_state(stmt).canonicalize();

                    stack_frame.update_state(*index, state.clone());
                    stack_frame.update_defs(stmt);

                    state
                }
                NodeType::SelfLoop => {
                    let state = NodeState::Executed;
                    stack_frame.update_state(*index, state.clone());
                    state
                }
            };

            let parents = model
                .as_ref()
                .neighbors_directed(*index, Direction::Incoming)
                .map(|parent| (model[parent], stack_frame.get_state(&parent)))
                .collect::<StateConf<'data>>();

            self.next_items
                .push_back(TraceItem::new(node, state, parents.canonicalize()));
        }

        if stmt.metadata.is_ret() {
            // This statement returns from a function,
            // hence we can throw associated stack frame away.
            self.stack_frames.pop();
        }

        // We cannot use just stmt.is_call() because static analysis in some cases would not detect
        // that the statement is call, especially in dynamic languages.
        if let Some(next_stmt) = self.trace.peek() {
            if !stmt.is_succ(next_stmt) {
                // Initialize new stack frame which will be used in the called function.
                self.stack_frames.push(StackFrame::new());
            }
        }

        self.next_items.pop_front()
    }
}

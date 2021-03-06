use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::iter::Peekable;

use petgraph::graph::{DefaultIx, NodeIndex};
use petgraph::Direction;

use super::models::{Model, Node, NodeType};
use crate::api::Api;
use crate::arena::{CheapOrd, P, S};
use crate::data::{access::AccessChain, statement::Statement, types::FuncName};
use crate::plugins::Rationale;
use crate::queries::{Pdg, Stmts};

// We need BTreeSet because we want to keep a unique order of the elements
// for NodeState's implementation of Hash trait.
type DataContext = BTreeSet<(AccessChain, CheapOrd<P<Statement>>)>;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone)]
pub enum NodeState {
    // Immediate successor on the path where the program flow went from the branching.
    Predicate(CheapOrd<P<Statement>>),
    // Variable and statement that defined the variable last.
    Data(DataContext),
    // When node has not been executed yet.
    NotExecuted,
    Executed,
}

impl NodeState {
    pub fn canonicalize(self) -> NodeState {
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

impl Hash for NodeState {
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

pub type StateConf = BTreeSet<(Node, NodeState)>;

pub trait StateConfExt {
    fn canonicalize(self) -> Option<Self>
    where
        Self: Sized;

    fn from_node(node: Node, state: NodeState) -> Self;
}

impl StateConfExt for StateConf {
    fn canonicalize(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }

    fn from_node(node: Node, state: NodeState) -> Self {
        [(node, state)].iter().cloned().collect()
    }
}

struct StackFrame<N: Hash + Eq> {
    pub current_states: HashMap<N, NodeState>,
    pub current_defs: HashMap<AccessChain, P<Statement>>,
}

impl<N: Hash + Eq + Copy> StackFrame<N> {
    pub fn new() -> Self {
        StackFrame {
            current_states: HashMap::new(),
            current_defs: HashMap::new(),
        }
    }

    pub fn update_state(&mut self, node: N, state: NodeState) {
        self.current_states.insert(node, state.canonicalize());
    }

    pub fn get_state(&self, node: &N) -> NodeState {
        self.current_states
            .get(node)
            .cloned()
            .unwrap_or(NodeState::NotExecuted)
    }

    pub fn get_data_state(&self, stmt: &P<Statement>) -> NodeState {
        let mut state = BTreeSet::new();

        for use_access in stmt
            .as_ref()
            .uses
            .iter()
            .map(|access| access.as_ref())
            .map(AccessChain::from_uses)
        {
            for (def_access, def) in self.current_defs.iter() {
                if use_access.influenced_by(def_access) {
                    state.insert((use_access.clone(), CheapOrd::new(*def)));
                }
            }
        }

        NodeState::Data(state)
    }

    pub fn update_defs(&mut self, stmt: P<Statement>) {
        // TODO: A data structure that tries to model data dependencies of pointers should be used.
        //       At least on a level, when a pointer is sent to a function and the function modifies it (or its child),
        //       then it should be added as a definition of the function call.
        for access in stmt
            .as_ref()
            .defs
            .iter()
            .map(|access| access.as_ref())
            .map(AccessChain::from_defs)
        {
            self.current_defs.insert(access, stmt);
        }
    }
}

pub struct TraceItem {
    pub node: Node,
    pub node_state: NodeState,
    pub parents_state_conf: Option<StateConf>,
}

impl TraceItem {
    pub fn new(node: Node, node_state: NodeState, parents_state_conf: Option<StateConf>) -> Self {
        TraceItem {
            node,
            node_state,
            parents_state_conf,
        }
    }

    pub fn explain(self, expected: Option<(Node, NodeState)>) -> Rationale {
        let mut rationale = Rationale::new();
        rationale.add_text("The element entered an unusual state.");

        let only_state = if let Some(expected) = &expected {
            if expected.0 == self.node && expected.1 == self.node_state {
                true
            } else {
                false
            }
        } else {
            false
        };

        match self.node_state {
            NodeState::Predicate(succ) => {
                rationale
                    .add_text(" The predicate outcome of ")
                    .add_anchor(self.node.stmt.as_ref().loc)
                    .add_text(" lead the execution unexpectedly to ")
                    .add_anchor(succ.as_ref().loc)
                    .add_text(".");
            }
            NodeState::Data(ctx) => {
                rationale
                    .add_text(" The values of the variables used by ")
                    .add_anchor(self.node.stmt.as_ref().loc)
                    .add_text(" were assigned by the following statements: ");

                let mut iter = ctx.into_iter();
                let mut mentioned = HashSet::new();

                if let Some((_, def)) = iter.next() {
                    let loc = def.as_ref().loc;
                    if !mentioned.contains(&loc) {
                        rationale.add_anchor(loc);
                        mentioned.insert(loc);
                    }
                }

                for (_, def) in iter {
                    rationale.add_text(", ").add_anchor(def.as_ref().loc);
                }

                rationale.add_text(".");
            }
            _ => {}
        }

        rationale.paragraph();

        if let Some(expected) = expected {
            if only_state {
                rationale.add_text("This is, however, the only state that the node encounters.");
            } else {
                rationale
                    .add_text("This is what was expected the most:")
                    .paragraph();
                Self::explain_node_state(expected.0, expected.1, &mut rationale);
            }
        } else {
            rationale.add_text("This is, however, the only state that the node encounters.");
        }

        if let Some(parents) = self.parents_state_conf {
            rationale
                .paragraph()
                .newline()
                .add_text("It all happened in these circumstances:");

            for parent in parents {
                rationale.paragraph();

                Self::explain_node_state(parent.0, parent.1, &mut rationale);
            }
        }

        rationale
    }

    fn explain_node_state(node: Node, state: NodeState, rationale: &mut Rationale) {
        rationale.add_text("- ");

        match state {
            NodeState::Predicate(succ) => {
                rationale
                    .add_text("The predicate outcome of ")
                    .add_anchor(node.stmt.as_ref().loc)
                    .add_text(" lead the execution to ")
                    .add_anchor(succ.as_ref().loc)
                    .add_text(".");
            }
            NodeState::Data(ctx) => {
                rationale
                    .add_text("The values of the variables used by ")
                    .add_anchor(node.stmt.as_ref().loc)
                    .add_text(" were assigned by the following statements: ");

                let mut iter = ctx.into_iter();

                if let Some((_, def)) = iter.next() {
                    rationale.add_anchor(def.as_ref().loc);
                }

                for (_, def) in iter {
                    rationale.add_text(", ").add_anchor(def.as_ref().loc);
                }

                rationale.add_text(".");
            }
            NodeState::Executed => {
                rationale
                    .add_text("Statement ")
                    .add_anchor(node.stmt.as_ref().loc)
                    .add_text(" was executed.");
            }
            NodeState::NotExecuted => {
                rationale
                    .add_text("Statement ")
                    .add_anchor(node.stmt.as_ref().loc)
                    .add_text(" was not executed.");
            }
        }
    }
}

pub struct Trace<'a, I: Iterator<Item = P<Statement>>, M: Model> {
    stack_frames: Vec<StackFrame<NodeIndex<DefaultIx>>>,
    trace: Peekable<I>,
    next_items: VecDeque<TraceItem>,
    api: &'a Api,
    models: HashMap<S<FuncName>, M>,
}

impl<'a, I: Iterator<Item = P<Statement>>, M: Model> Trace<'a, I, M> {
    pub fn new(trace: I, api: &'a Api) -> Self {
        Trace {
            stack_frames: vec![StackFrame::new()],
            trace: trace.peekable(),
            next_items: VecDeque::with_capacity(2),
            api,
            models: HashMap::new(),
        }
    }
}

impl<'a, I: Iterator<Item = P<Statement>>, M: Model> Iterator for Trace<'a, I, M> {
    type Item = TraceItem;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.next_items.is_empty() {
            return self.next_items.pop_front();
        }

        let stmts = self.api.query::<Stmts>().unwrap();

        let stmt_ptr = self.trace.next()?;
        let stmt = stmt_ptr.as_ref();

        let func = stmts.find_fn(&stmt.id).unwrap();

        // There should always exist a stack frame. If there is not, then one of the following happened:
        //   * The function from top-level stack frame returned
        //     and there exists a statement in the trace that follows it.
        //   * We missed a function call and a return statement discarded wrong stack frame.
        let stack_frame = self.stack_frames.last_mut().unwrap();

        let pdg = self.api.query_with::<Pdg>(func).unwrap();

        // Get (or initialize) model for the function which the statement comes from.
        let model = self
            .models
            .entry(*func)
            .or_insert_with(|| M::from_pdg(&pdg))
            .get_graph();

        // Get all nodes from the model corresponding to the statement.
        for index in model[&stmt_ptr].iter() {
            let node = model[*index];

            let state = match node.typ {
                NodeType::Predicate => {
                    // Predicate node must have a successor, so we can unwrap.
                    let next = self.trace.peek().unwrap();
                    let state = NodeState::Predicate(CheapOrd::new(*next));

                    stack_frame.update_state(*index, state.clone());

                    state
                }
                NodeType::NonPredicate => {
                    let state = stack_frame.get_data_state(&stmt_ptr).canonicalize();

                    stack_frame.update_state(*index, state.clone());
                    stack_frame.update_defs(stmt_ptr);

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
                .collect::<StateConf>();

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
            if !stmt.is_succ(next_stmt.as_ref()) {
                // Initialize new stack frame which will be used in the called function.
                self.stack_frames.push(StackFrame::new());
            }
        }

        self.next_items.pop_front()
    }
}

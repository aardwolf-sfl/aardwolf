use std::collections::HashMap;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::plugins::{AardwolfPlugin, LocalizationItem, PluginInitError, Rationale};
use crate::raw::data::TestStatus;

pub struct ProbGraph;

impl AardwolfPlugin for ProbGraph {
    fn init<'a>(_api: &'a Api<'a>, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(ProbGraph)
    }

    fn run_loc<'a, 'b>(&'b self, api: &'a Api<'a>) -> Vec<LocalizationItem<'b>> {
        let stmts = api.get_stmts().unwrap();
        let tests = api.get_tests().unwrap();
        let ppdg = api.make::<ppdg::Ppdg>().unwrap();

        let failing = tests
            .iter_statuses()
            .find(|(name, status)| **status == TestStatus::Failed)
            .map(|(name, status)| name)
            .unwrap();

        let mut probs_with_states = HashMap::new();

        let trace = ppdg::NodeStateTrace::new(tests.iter_stmts(failing).unwrap(), stmts);

        for (index, (current, parent)) in trace.enumerate() {
            // Consider only nodes with a parent.
            if let Some(parent) = parent {
                let lowest_prob = probs_with_states
                    .get(current.stmt)
                    .map(|(prob, _, _, _)| *prob)
                    .unwrap_or(std::f32::MAX);

                let prob = ppdg.get_prob(
                    current.stmt,
                    current.state.clone(),
                    parent.stmt,
                    parent.state.clone(),
                );

                if prob < lowest_prob {
                    probs_with_states
                        .insert(current.stmt, (prob, current.state, parent.state, index));
                }
            }
        }

        let mut default_rationale = Rationale::new();
        default_rationale.add_text(
            "The statement enters to an unusual state given the state of its parents control flow.",
        );

        let mut results = probs_with_states.into_iter().collect::<Vec<_>>();

        // Sort the results by index. If there are some ties in score,
        // this will prioritizes statements that occur earlier.
        results.sort_unstable_by(|lhs, rhs| (lhs.1).3.cmp(&(rhs.1).3));

        results
            .into_iter()
            .map(|(stmt, (prob, state, parent_state, _))| {
                if let Some(expected) = ppdg.get_expected_state(stmt, &state) {
                    let mut rationale = default_rationale.clone();
                    match expected.state {
                        ppdg::State::Predicate(succ) => {
                            rationale
                                .add_text(" Expected control flow of ")
                                .add_anchor(expected.stmt.loc)
                                .add_text(" is ")
                                .add_anchor(succ.loc)
                                .add_text(", not this statement.");
                        }
                        ppdg::State::Data(ctx) => {
                            rationale
                                .add_text(" Expected data flow of ")
                                .add_anchor(expected.stmt.loc)
                                .add_text(" is ");

                            match parent_state {
                                ppdg::State::Data(ctx2) => {
                                    use ppdg::DataContextExt;
                                    let diff = ctx.diff(&ctx2);
                                    let mut diff_iter = diff.iter();

                                    if let Some((_, expected, actual)) = diff_iter.next() {
                                        rationale
                                            .add_anchor(expected.loc)
                                            .add_text(" (not ")
                                            .add_anchor(actual.loc)
                                            .add_text(")");
                                    } else {
                                        // What else to do?
                                        rationale.add_text("different");
                                    }

                                    for (_, expected, actual) in diff_iter {
                                        rationale
                                            .add_text(", ")
                                            .add_anchor(expected.loc)
                                            .add_text(" (not ")
                                            .add_anchor(actual.loc)
                                            .add_text(")");
                                    }
                                }
                                _ => {
                                    let mut ctx_iter = ctx.iter();
                                    if let Some((_, stmt)) = ctx_iter.next() {
                                        rationale.add_anchor(stmt.loc);
                                    } else {
                                        // What else to do?
                                        rationale.add_text("different");
                                    }

                                    for (_, stmt) in ctx_iter {
                                        rationale.add_text(", ").add_anchor(stmt.loc);
                                    }
                                }
                            }
                        }
                    }

                    rationale.add_text(".");
                    LocalizationItem::new(stmt.loc, 1.0 - prob, rationale.clone()).unwrap()
                } else {
                    LocalizationItem::new(stmt.loc, 1.0 - prob, default_rationale.clone()).unwrap()
                }
            })
            .collect()
    }
}

mod ppdg {
    use std::collections::{BTreeSet, HashMap, HashSet};
    use std::hash::{Hash, Hasher};
    use std::iter::Peekable;

    use crate::api::Api;
    use crate::raw::data::{Data, Statement, TestName};
    use crate::structures::{FromRawData, FromRawDataError, Stmts};

    // We need BTreeSet because we want to keep a unique order of the elements
    // for State's implementation of Hash trait.
    type DataContext<'a> = BTreeSet<(u64, &'a Statement)>;

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum State<'a> {
        // Immediate successor on the path where the program flow went from the branching.
        Predicate(&'a Statement),
        // Variable and statement that defined the variable last.
        Data(DataContext<'a>),
    }

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

    impl<'a> Hash for State<'a> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            match self {
                State::Predicate(stmt) => stmt.hash(state),
                State::Data(context) => {
                    // Context is most likely very small set,
                    // so the hashing operation is constant-time
                    // (at least in majority of cases).
                    for item in context.iter() {
                        item.hash(state);
                    }
                }
            }
        }
    }

    type Counter<T: Hash + Eq> = HashMap<T, usize>;

    trait CounterExt<T> {
        fn inc(&mut self, value: T);
        fn merge(self, other: Self) -> Self;
    }

    impl<T: Hash + Eq> CounterExt<T> for Counter<T> {
        fn inc(&mut self, value: T) {
            *self.entry(value).or_insert(0) += 1;
        }

        fn merge(mut self, other: Self) -> Self {
            for (value, count) in other {
                *self.entry(value).or_insert(0) += count;
            }

            self
        }
    }

    struct StackFrame<'a> {
        pub parent: Option<(&'a Statement, State<'a>)>,
        pub data_context: HashMap<u64, &'a Statement>,
    }

    impl<'a> StackFrame<'a> {
        pub fn new() -> Self {
            StackFrame {
                parent: None,
                data_context: HashMap::new(),
            }
        }

        pub fn get_data_state(&self, stmt: &'a Statement) -> State<'a> {
            let mut state = BTreeSet::new();

            for var in stmt.uses.iter().flat_map(|access| access.get_scalars()) {
                if let Some(def) = self.data_context.get(&var) {
                    state.insert((var, *def));
                }
            }

            State::Data(state)
        }

        pub fn update_data_context(&mut self, stmt: &'a Statement) {
            // TODO: A data structure that tries to model data dependencies of pointers should be used.
            //       At least on a level, when a pointer is sent to a function and the function modifies it (or its child),
            //       then it should be added as a definition of the function call.
            for var in stmt.defs.iter().flat_map(|access| access.get_scalars()) {
                self.data_context.insert(var, stmt);
            }
        }
    }

    pub struct Ppdg<'a>(HashMap<(&'a Statement, State<'a>, &'a Statement, State<'a>), f32>);

    impl<'a> FromRawData<'a> for Ppdg<'a> {
        fn from_raw(data: &'a Data, api: &'a Api<'a>) -> Result<Self, FromRawDataError> {
            let tests = api.get_tests().unwrap();
            let stmts = api.get_stmts().unwrap();

            let mut occurence_counter = Counter::new();
            let mut state_counter = Counter::new();
            let mut conditional_state_counter = Counter::new();

            // Learn PPDG on passing tests.
            for test in tests.iter_names().filter(|name| tests.is_passed(name)) {
                let trace = NodeStateTrace::new(tests.iter_stmts(test).unwrap(), stmts);

                for (current, parent) in trace {
                    occurence_counter.inc(current.stmt);
                    state_counter.inc((current.stmt, current.state.clone()));
                    if let Some(parent) = parent {
                        conditional_state_counter.inc((
                            current.stmt,
                            current.state,
                            parent.stmt,
                            parent.state,
                        ));
                    }
                }
            }

            // Compute probabilities only of nodes that have a parent.
            let probs = conditional_state_counter
                .into_iter()
                .map(
                    |((stmt, stmt_state, parent, parent_state), conditional_count)| {
                        let state_count =
                            *state_counter.get(&(parent, parent_state.clone())).unwrap();
                        let prob = ((conditional_count as f64) / (state_count as f64)) as f32;
                        ((stmt, stmt_state, parent, parent_state), prob)
                    },
                )
                .collect();

            Ok(Ppdg(probs))
        }
    }

    impl<'a> Ppdg<'a> {
        pub fn get_prob(
            &self,
            stmt: &'a Statement,
            stmt_state: State<'a>,
            parent: &'a Statement,
            parent_state: State<'a>,
        ) -> f32 {
            *self
                .0
                .get(&(stmt, stmt_state, parent, parent_state))
                .unwrap_or(&0.0)
        }

        pub fn get_expected_state(
            &self,
            stmt: &'a Statement,
            state: &'a State<'a>,
        ) -> Option<NodeState<'a>> {
            self.0
                .iter()
                .filter(|((item_stmt, item_state, _, _), _)| {
                    *item_stmt == stmt && item_state == state
                })
                .max_by(|(_, lhs_prob), (_, rhs_prob)| rhs_prob.partial_cmp(lhs_prob).unwrap())
                .map(|((_, _, parent, parent_state), _)| {
                    NodeState::new(parent, parent_state.clone())
                })
        }
    }

    pub struct NodeStateTrace<'a, I: Iterator<Item = &'a Statement>> {
        parent: Option<(&'a Statement, State<'a>)>,
        stack_frames: Vec<StackFrame<'a>>,
        trace: Peekable<I>,
        next_item: Option<(NodeState<'a>, Option<NodeState<'a>>)>,

        stmts: &'a Stmts<'a>,
    }

    impl<'a, I: Iterator<Item = &'a Statement>> NodeStateTrace<'a, I> {
        pub fn new(raw_trace: I, stmts: &'a Stmts<'a>) -> Self {
            NodeStateTrace {
                parent: None,
                stack_frames: vec![StackFrame::new()],
                trace: raw_trace.peekable(),
                next_item: None,
                stmts,
            }
        }
    }

    #[derive(Clone)]
    pub struct NodeState<'a> {
        pub stmt: &'a Statement,
        pub state: State<'a>,
    }

    impl<'a> NodeState<'a> {
        pub fn new(stmt: &'a Statement, state: State<'a>) -> Self {
            NodeState { stmt, state }
        }
    }

    impl<'a, I: Iterator<Item = &'a Statement>> Iterator for NodeStateTrace<'a, I> {
        type Item = (NodeState<'a>, Option<NodeState<'a>>);

        fn next(&mut self) -> Option<Self::Item> {
            if self.next_item.is_some() {
                return self.next_item.take();
            }

            // FIXME: Everywhere we clone values from or to self.parent,
            //        we could probably just take the value of leaving the option empty?

            let stmt = self.trace.next()?;
            let func = self.stmts.find_fn(stmt).unwrap();

            // There should always exist a stack frame. If there is not, then one of the following happened:
            //   * The function from top-level stack frame returned
            //     and there exists a statement in the trace that follows it.
            //   * We missed a function call and a return statement discarded wrong stack frame.
            let stack_frame = self.stack_frames.last_mut().unwrap();

            let data_state = stack_frame.get_data_state(stmt);
            let node_state = NodeState::new(stmt, data_state.clone());

            // FIXME: The following snippet does not follow PPDG rules in case of def-use self-loops.
            let parent_node_state = self
                .parent
                .clone()
                .map(|(stmt, state)| NodeState::new(stmt, state));

            // Assign the parent variable to be used in the next loop iteration.
            self.parent = Some((stmt, data_state.clone()));

            // Update definition of variables that this statement defines.
            stack_frame.update_data_context(stmt);

            // If the statement is a predicate, create also its predicate state.
            if stmt.is_predicate() {
                // Predicate node must have a successor, so we can unwrap.
                let next = self.trace.peek().unwrap();
                let predicate_state = State::Predicate(next);
                let next_node_state = NodeState::new(stmt, predicate_state.clone());

                // Store the item which should be returned in the next iteration.
                self.next_item = Some((next_node_state, Some(node_state.clone())));

                // Reassign the parent variable by predicate state instead of data state.
                // We can do this because the next iteration will return the `self.next_item`
                // where the parent of the node is already set appropriately.
                self.parent = Some((stmt, predicate_state));
            }

            if stmt.is_ret() {
                // This statement returns from a function,
                // hence we can throw associated stack frame away.
                self.stack_frames.pop();

                // We need to restore the parent valid in the parent stack frame.
                // The frame should always exist except when the return statement is from top-level stack frame
                // and there is no other statement in the trace.
                self.parent = self
                    .stack_frames
                    .last()
                    .and_then(|frame| frame.parent.clone());
            }

            // FIXME: This will not work with things like operator overloading, when a custom function is called
            //        but it cannot be determined using static analysis in the frontend for some reason
            //        (especially true for dynamic languages). But we should be able to determine a call
            //        by looking at the next statement - the statement is a call if the next one is not the statement's successor.
            if stmt.is_call() {
                // Store current parent to be accessed later, and reset the local variable for parent.
                self.stack_frames.last_mut().unwrap().parent = self.parent.clone();
                self.parent = None;

                // Initialize new stack frame which will be used in the called function.
                self.stack_frames.push(StackFrame::new());
            }

            Some((node_state, parent_node_state))
        }
    }
}

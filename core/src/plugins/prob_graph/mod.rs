mod models;
mod trace;

use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;

use yaml_rust::Yaml;

use self::models::*;
use self::trace::*;
use crate::api::Api;
use crate::plugins::{AardwolfPlugin, Preprocessing, PluginError, PluginInitError, Results};
use crate::queries::Tests;

enum ModelType {
    Dependence,
    Bayesian,
}

pub struct ProbGraph {
    model: ModelType,
}

impl AardwolfPlugin for ProbGraph {
    fn init(_api: &Api, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        let model = match opts.get("model").and_then(|model| model.as_str()) {
            Some("dependency") => ModelType::Dependence,
            Some("bayesian") => ModelType::Bayesian,
            None => ModelType::Dependence,
            Some(unknown) => return Err(format!("Unknown model '{}'.", unknown)),
        };

        Ok(ProbGraph { model })
    }

    fn run_loc(
        &self,
        api: &Api,
        results: &mut Results,
        _preprocessing: &Preprocessing,
    ) -> Result<(), PluginError> {
        match self.model {
            ModelType::Dependence => self.run_loc_typed::<DependencyNetwork>(api, results),
            ModelType::Bayesian => self.run_loc_typed::<BayesianNetwork>(api, results),
        }
    }
}

impl ProbGraph {
    pub fn run_loc_typed<M: Model>(
        &self,
        api: &Api,
        results: &mut Results,
    ) -> Result<(), PluginError> {
        let tests = api.query::<Tests>()?;

        let ppdg = self.learn_ppdg::<M>(api);

        for test in tests.iter_failed() {
            let trace: Trace<_, M> = Trace::new(tests.iter_stmts(test).unwrap().copied(), api);
            M::run_loc(trace, &ppdg, api, results)?;
        }

        Ok(())
    }

    pub fn learn_ppdg<M: Model>(&self, api: &Api) -> Ppdg {
        let tests = api.query::<Tests>().unwrap();
        let mut ppdg = Ppdg::new();

        // Learn PPDG on passing tests.
        for test in tests.iter_passed() {
            // We don't filter irrelevant statements because it might negatively affect the parent state computation.
            let trace: Trace<_, M> = Trace::new(tests.iter_stmts(test).unwrap().copied(), api);

            for item in trace {
                // Increment n(X)
                ppdg.inc_occurrence(item.node);

                // Increment n(X = x)
                ppdg.inc_state_conf(StateConf::from_node(item.node, item.node_state.clone()));

                if let Some(mut parents_state_conf) = item.parents_state_conf {
                    // Increment n(Pa(X) = pa)
                    ppdg.inc_state_conf(parents_state_conf.clone());

                    // Increment n(X = x, Pa(X) = pa)
                    parents_state_conf.insert((item.node, item.node_state));
                    ppdg.inc_state_conf(parents_state_conf);
                }
            }
        }

        ppdg
    }
}

type Counter<T> = HashMap<T, usize>;

trait CounterExt<T> {
    fn inc(&mut self, value: T);
    fn get_safe(&self, value: &T) -> usize;
    fn merge(self, other: Self) -> Self;
}

impl<T: Hash + Eq> CounterExt<T> for Counter<T> {
    fn inc(&mut self, value: T) {
        *self.entry(value).or_insert(0) += 1;
    }

    fn get_safe(&self, value: &T) -> usize {
        *self.get(value).unwrap_or(&0)
    }

    fn merge(mut self, other: Self) -> Self {
        for (value, count) in other {
            *self.entry(value).or_insert(0) += count;
        }

        self
    }
}

pub struct Ppdg {
    occurrence_counter: Counter<Node>,
    state_conf_counter: Counter<StateConf>,
    node_states: HashMap<Node, BTreeSet<NodeState>>,
}

impl Ppdg {
    pub fn new() -> Self {
        Ppdg {
            occurrence_counter: Counter::new(),
            state_conf_counter: Counter::new(),
            node_states: HashMap::new(),
        }
    }

    pub fn inc_occurrence(&mut self, node: Node) {
        self.occurrence_counter.inc(node);
    }

    pub fn inc_state_conf(&mut self, conf: StateConf) {
        for (node, state) in &conf {
            self.node_states
                .entry(node.clone())
                .or_insert(BTreeSet::new())
                .insert(state.clone());
        }

        self.state_conf_counter.inc(conf);
    }

    pub fn get_prob(&self, item: &TraceItem) -> f32 {
        let (nom, denom) = if let Some(parents_state_conf) = &item.parents_state_conf {
            let mut joint = parents_state_conf.clone();
            joint.insert((item.node.clone(), item.node_state.clone()));

            let parents = self.state_conf_counter.get_safe(parents_state_conf);
            let joint = self.state_conf_counter.get_safe(&joint);

            (joint, parents)
        } else {
            let node_in_state = self.state_conf_counter.get_safe(&StateConf::from_node(
                item.node.clone(),
                item.node_state.clone(),
            ));

            let node_occurrence = self.occurrence_counter.get_safe(&item.node);

            (node_in_state, node_occurrence)
        };

        match (nom, denom) {
            (0, _) | (_, 0) => 0.0,
            (a, b) => ((a as f64) / (b as f64)) as f32,
        }
    }

    pub fn node_states(&self, node: &Node) -> Option<&BTreeSet<NodeState>> {
        self.node_states.get(node)
    }
}

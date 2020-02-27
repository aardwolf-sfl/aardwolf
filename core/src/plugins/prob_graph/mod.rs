mod graph_ext;
mod models;
mod pdg;
mod trace;

use std::collections::HashMap;
use std::hash::Hash;

use yaml_rust::Yaml;

use self::models::*;
use self::trace::*;
use crate::api::Api;
use crate::plugins::{AardwolfPlugin, LocalizationItem, PluginInitError, Rationale};
use crate::raw::data::TestStatus;
use crate::structures::{FromRawData, FromRawDataError};

enum ModelType {
    Dependence,
    Bayesian,
}

pub struct ProbGraph {
    model: ModelType,
}

impl AardwolfPlugin for ProbGraph {
    fn init<'a>(_api: &'a Api<'a>, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
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

    fn run_loc<'a, 'b>(&'b self, api: &'a Api<'a>) -> Vec<LocalizationItem<'a, 'b>> {
        match self.model {
            ModelType::Dependence => {
                let ppdg = self.learn_ppdg::<DependencyNetwork>(api);
                self.process_failing::<DependencyNetwork>(&ppdg, api)
            }
            ModelType::Bayesian => {
                let ppdg = self.learn_ppdg::<BayesianNetwork>(api);
                self.process_failing::<BayesianNetwork>(&ppdg, api)
            }
        }
    }
}

impl ProbGraph {
    pub fn learn_ppdg<'a, M: Model<'a>>(&self, api: &'a Api<'a>) -> Ppdg<'a> {
        let tests = api.get_tests();
        let mut ppdg = Ppdg::new();

        // Learn PPDG on passing tests.
        for test in tests.iter_names().filter(|name| tests.is_passed(name)) {
            let trace: Trace<_, M> = Trace::new(tests.iter_stmts(test).unwrap(), api);

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

    pub fn process_failing<'a, 'b, M: Model<'a>>(
        &self,
        ppdg: &Ppdg<'a>,
        api: &'a Api<'a>,
    ) -> Vec<LocalizationItem<'a, 'b>> {
        let stmts = api.get_stmts();
        let tests = api.get_tests();

        let failing = tests
            .iter_statuses()
            .find(|(name, status)| **status == TestStatus::Failed)
            .map(|(name, status)| name)
            .unwrap();

        let mut probs = HashMap::new();

        let trace: Trace<_, M> = Trace::new(tests.iter_stmts(failing).unwrap(), api);

        for (index, item) in trace.enumerate() {
            let prob = ppdg.get_prob(&item);

            let lowest_prob = probs
                .get(item.node.stmt)
                .map(|(prob, _)| *prob)
                .unwrap_or(std::f32::MAX);

            if prob < lowest_prob {
                probs.insert(item.node.stmt, (prob, index));
            }
        }

        let mut default_rationale = Rationale::new();
        default_rationale.add_text(
            "The statement enters to an unusual state given the state of its control and data dependencies.",
        );

        let mut results = probs.into_iter().collect::<Vec<_>>();

        // Sort the results by index. If there are some ties in score,
        // this will prioritizes statements that occur earlier.
        results.sort_unstable_by(|lhs, rhs| (lhs.1).1.cmp(&(rhs.1).1));

        results
            .into_iter()
            .map(|(stmt, (prob, _))| {
                LocalizationItem::new(stmt.loc, stmt, 1.0 - prob, default_rationale.clone())
                    .unwrap()
            })
            .collect()
    }
}

type Counter<T: Hash + Eq> = HashMap<T, usize>;

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

pub struct Ppdg<'a> {
    occurrence_counter: Counter<Node<'a>>,
    state_conf_counter: Counter<StateConf<'a>>,
}

impl<'a> Ppdg<'a> {
    pub fn new() -> Self {
        Ppdg {
            occurrence_counter: Counter::new(),
            state_conf_counter: Counter::new(),
        }
    }

    pub fn inc_occurrence(&mut self, node: Node<'a>) {
        self.occurrence_counter.inc(node);
    }

    pub fn inc_state_conf(&mut self, conf: StateConf<'a>) {
        self.state_conf_counter.inc(conf);
    }

    pub fn get_prob(&self, item: &TraceItem<'a>) -> f32 {
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
}

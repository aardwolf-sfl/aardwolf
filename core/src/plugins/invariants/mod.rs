mod detector;

use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

use yaml_rust::Yaml;

use crate::api::Api;
use crate::plugins::{AardwolfPlugin, LocalizationItem, PluginInitError, Rationale};
use crate::raw::data::{Access, TestName, TestStatus, VariableData, VariableDataType};

use detector::Stats;

// TODO: This should perhaps globally available macro.
macro_rules! required {
    ($structure:expr) => {
        match $structure {
            Some(structure) => structure,
            None => return Vec::new(),
        }
    };
}

pub struct Invariants;

impl AardwolfPlugin for Invariants {
    fn init<'a>(_api: &'a Api<'a>, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(Invariants)
    }

    fn run_loc<'a, 'b>(&'b self, api: &'a Api<'a>) -> Vec<LocalizationItem<'a, 'b>> {
        let stmts = api.get_stmts();
        let tests = api.get_tests();
        let vars = required!(api.get_vars());

        let mut stats = Stats::new();

        for test in tests.iter_names().filter(|name| tests.is_passed(name)) {
            for item in vars.iter_vars(test).unwrap() {
                for (access, data) in item.zip() {
                    stats.learn(access, data, test);
                }
            }
        }

        let failing = tests
            .iter_statuses()
            .find(|(name, status)| **status == TestStatus::Failed)
            .map(|(name, status)| name)
            .unwrap();

        let mut results = Vec::new();

        for item in vars.iter_vars(failing).unwrap() {
            for (access, data) in item.zip() {
                let violations = stats.check(data, access);

                if !violations.is_empty() {
                    let confidence = violations
                        .iter()
                        .map(|info| info.confidence)
                        // Confidence must be a finite number.
                        .max_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap())
                        // We checked that the vector is not empty.
                        .unwrap();

                    let mut explanation = violations
                        .iter()
                        .map(|info| info.explain(data))
                        .collect::<Vec<_>>()
                        .join(", ");

                    explanation.push('.');

                    let mut rationale = Rationale::new();

                    // NOTE: Could be configurable to disable args, calls, etc.
                    if item.stmt.is_arg() {
                        rationale.add_text("The value of this argument violates some invariants inferred from passing runs.");
                    } else if item.stmt.is_ret() {
                        rationale.add_text(
                            "The return value violates some invariants inferred from passing runs.",
                        );
                    } else if item.stmt.is_call() {
                        rationale.add_text("The result of this function call violates some invariants inferred from passing runs.");
                    } else {
                        rationale.add_text("The result of this statement violates some invariants inferred from passing runs.");
                    }

                    rationale
                        .add_text(" The violations are: ")
                        .add_text(explanation);

                    results.push(
                        LocalizationItem::new(item.stmt.loc, item.stmt, confidence, rationale)
                            .unwrap(),
                    );
                }
            }
        }

        results
    }
}

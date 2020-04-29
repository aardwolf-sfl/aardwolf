mod detector;

use std::collections::HashMap;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::plugins::{
    AardwolfPlugin, IrrelevantItems, LocalizationItem, MissingApi, PluginError, PluginInitError,
    Rationale, Results,
};

use detector::Stats;

// TODO: This should perhaps globally available macro.
macro_rules! required {
    ($structure:expr, $name:expr) => {
        match $structure {
            Some(structure) => structure,
            None => return Err(PluginError::MissingApi($name)),
        }
    };
}

pub struct Invariants;

impl AardwolfPlugin for Invariants {
    fn init<'data>(
        _api: &'data Api<'data>,
        _opts: &HashMap<String, Yaml>,
    ) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(Invariants)
    }

    fn run_loc<'data, 'param>(
        &self,
        api: &'data Api<'data>,
        results: &'param mut Results<'data>,
        irrelevant: &'param IrrelevantItems,
    ) -> Result<(), PluginError> {
        let tests = api.get_tests();
        let vars = required!(api.get_vars(), MissingApi::Vars);

        let mut stats = Stats::new();

        for test in tests.iter_passed() {
            for item in vars
                .iter_vars(test)
                .unwrap()
                .filter(|item| irrelevant.is_stmt_relevant(item.stmt))
            {
                for (access, data) in item.zip() {
                    stats.learn(access, data, test.clone());
                }
            }
        }

        for test in tests.iter_failed() {
            for item in vars.iter_vars(test).unwrap() {
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
                        if item.stmt.metadata.is_arg() {
                            rationale.add_text("The value of this argument violates some invariants inferred from passing runs.");
                        } else if item.stmt.metadata.is_call() {
                            rationale.add_text("The result of this function call violates some invariants inferred from passing runs.");
                        } else {
                            rationale.add_text("The result of this statement violates some invariants inferred from passing runs.");
                        }

                        rationale
                            .add_text(" The violations are: ")
                            .add_text(explanation);

                        results.add(
                            LocalizationItem::new(item.stmt.loc, item.stmt, confidence, rationale)
                                .unwrap(),
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

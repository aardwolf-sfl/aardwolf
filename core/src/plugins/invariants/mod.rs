mod detector;

use std::collections::HashMap;

use yaml_rust::Yaml;

use crate::api::Api;
use crate::plugins::{
    AardwolfPlugin, Preprocessing, LocalizationItem, PluginError, PluginInitError, Rationale,
    Results,
};
use crate::queries::{Tests, Vars};

use detector::Stats;

pub struct Invariants;

impl AardwolfPlugin for Invariants {
    fn init(_api: &Api, _opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(Invariants)
    }

    fn run_loc(
        &self,
        api: &Api,
        results: &mut Results,
        preprocessing: &Preprocessing,
    ) -> Result<(), PluginError> {
        let tests = api.query::<Tests>()?;

        let mut stats = Stats::new();

        for test in tests.iter_passed() {
            let vars = api.query_with::<Vars>(test)?;

            for item in vars
                .iter()
                .filter(|item| preprocessing.is_stmt_relevant(item.stmt.as_ref()))
            {
                for (access, data) in item.zip() {
                    stats.learn(*access, *data, test.clone());
                }
            }
        }

        for test in tests.iter_failed() {
            let vars = api.query_with::<Vars>(test)?;

            for item in vars.iter() {
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

                        let stmt = item.stmt.as_ref();

                        // NOTE: Could be configurable to disable args, calls, etc.
                        if stmt.metadata.is_arg() {
                            rationale.add_text("The value of this argument violates some invariants inferred from passing runs.");
                        } else if stmt.metadata.is_call() {
                            rationale.add_text("The result of this function call violates some invariants inferred from passing runs.");
                        } else {
                            rationale.add_text("The result of this statement violates some invariants inferred from passing runs.");
                        }

                        rationale
                            .add_text(" The violations are: ")
                            .add_text(explanation);

                        results.add(
                            LocalizationItem::new(stmt.loc, item.stmt, confidence, rationale)
                                .unwrap(),
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

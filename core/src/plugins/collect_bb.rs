use std::collections::HashMap;

use petgraph::{
    graph::{DiGraph, NodeIndex},
    Direction,
};
use yaml_rust::Yaml;

use crate::api::Api;
use crate::arena::P;
use crate::data::statement::{Loc, Statement};
use crate::plugins::{
    AardwolfPlugin, LocalizationItem, NormalizedResults, PluginError, PluginInitError, Results,
};

pub struct CollectBb {
    plugin: String,
}

impl AardwolfPlugin for CollectBb {
    fn init<'data>(_api: &'data Api, opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        let plugin = match opts.get("for") {
            Some(Yaml::String(plugin)) => plugin.clone(),
            Some(_) => return Err(String::from("Invalid type of option \"for\"")),
            None => {
                return Err(String::from(
                    "Missing \"for\" option for specifying plugins to collect.",
                ))
            }
        };

        Ok(CollectBb { plugin })
    }

    fn run_post<'data, 'param>(
        &self,
        api: &'data Api,
        base: &'param HashMap<&'param str, &'param NormalizedResults>,
        results: &'param mut Results,
    ) -> Result<(), PluginError> {
        let mut original = base
            .get(self.plugin.as_str())
            .copied()
            .ok_or(PluginError::Inner(format!(
                "Results for \"{}\" not found.",
                self.plugin
            )))?
            .iter()
            .rev()
            .collect::<Vec<_>>();

        let stmts = api.get_stmts();
        let cfgs = api.get_cfgs();

        while let Some(item) = original.pop() {
            if let Some(cfg) = stmts
                .find_fn(item.root_stmt.as_ref())
                .and_then(|func| cfgs.get(&func))
            {
                // Find index in CFG corresponding to the statement.
                let index = cfg
                    .node_indices()
                    .find(|index| cfg[*index] == item.root_stmt)
                    .unwrap();

                // Create new location which will spread over all merged items.
                let mut loc = item.loc.clone();

                // Start at statement's index.
                let mut current = index;

                // Extend the location in the direction of control flow.
                loop {
                    // Get all neighbors in given direction.
                    let mut neighbors = cfg.neighbors_directed(current, Direction::Outgoing);

                    // If the number of neighbors is not one, then the statement is the terminator
                    // (ie., last statement) of the basic block.
                    if let Some(neighbor) = neighbors.next() {
                        if neighbors.next().is_some() {
                            break;
                        }

                        if self.extend_loc(&mut loc, &mut original, cfg, item, neighbor) {
                            // Continue with the neighbor.
                            current = neighbor;
                        } else {
                            // Stop extending.
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Start again at the original statement's index.
                current = index;

                // Extend the location in the direction opposed to control flow.
                loop {
                    // Get all neighbors in given direction.
                    let mut neighbors = cfg.neighbors_directed(current, Direction::Incoming);

                    // If the number of neighbors is not one, then the statement is the terminator
                    // (ie., last statement) of the basic block.
                    if let Some(neighbor) = neighbors.next() {
                        if neighbors.next().is_some() {
                            break;
                        }

                        if cfg.neighbors(neighbor).count() > 1 {
                            // The neighbor is a predicate statement, so it should not be included in the basic block.
                            break;
                        }

                        if self.extend_loc(&mut loc, &mut original, cfg, item, neighbor) {
                            // Continue with the neighbor.
                            current = neighbor;
                        } else {
                            // Stop extending.
                            break;
                        }
                    } else {
                        break;
                    }
                }

                results.add(
                    LocalizationItem::new(loc, item.root_stmt, item.score, item.rationale.clone())
                        .unwrap(),
                );
            }
        }

        Ok(())
    }
}

impl CollectBb {
    fn extend_loc(
        &self,
        loc: &mut Loc,
        original: &mut Vec<&LocalizationItem>,
        cfg: &DiGraph<P<Statement>, ()>,
        item: &LocalizationItem,
        index: NodeIndex,
    ) -> bool {
        let len = original.len();

        // Find all statements in the original results that belong to the neighbor statement and remove them.
        original.retain(|other| {
            // The localization item must include neighbor's location,
            // Merge only items which have equal score and the same rationale.
            if other.loc.contains(&cfg[index].as_ref().loc)
                && other.score == item.score
                && other.rationale == item.rationale
            {
                // Extend the location.
                *loc = loc.merge(&cfg[index].as_ref().loc);
                false
            } else {
                true
            }
        });

        if original.len() == len {
            // We did not find any item in original result that correspond to the neighbor statement,
            // so we stop the location extending here.
            false
        } else {
            // Continue with location extending.
            true
        }
    }
}

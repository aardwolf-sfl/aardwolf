use std::collections::HashMap;

use petgraph::graph::DiGraph;
use petgraph::Direction;

use crate::api::Api;
use crate::arena::{Arena, Dummy};
use crate::arena::{P, S};
use crate::data::{statement::Statement, types::FuncName, RawData};
use crate::structures::{FromRawData, FromRawDataError};

pub type Cfg = DiGraph<P<Statement>, ()>;

pub struct Cfgs(HashMap<S<FuncName>, Cfg>);

impl Cfgs {
    pub fn get(&self, func: &S<FuncName>) -> Option<&Cfg> {
        self.0.get(func)
    }

    pub fn entry() -> P<Statement> {
        Arena::dummy(Dummy::D1)
    }

    pub fn exit() -> P<Statement> {
        Arena::dummy(Dummy::D2)
    }
}

impl FromRawData for Cfgs {
    fn from_raw(data: &RawData, _api: &Api) -> Result<Self, FromRawDataError> {
        let mut result = HashMap::new();

        for (func_name, func_body) in data.modules.functions.iter() {
            let mut graph = DiGraph::with_capacity(func_body.len() + 1, func_body.len() + 1);
            let mut id_map = HashMap::new();

            for (id, stmt) in func_body.iter() {
                id_map.insert(id, graph.add_node(*stmt));
            }

            let entry = graph.add_node(Cfgs::entry());
            let exit = graph.add_node(Cfgs::exit());

            for (id, stmt) in func_body.iter() {
                let stmt = stmt.as_ref();

                for succ in stmt.succ.iter() {
                    graph.add_edge(id_map[id], id_map[succ], ());
                }

                if stmt.succ.is_empty() {
                    graph.add_edge(id_map[id], exit, ());
                }
            }

            // Connect the ENTRY node to all nodes without any predecessors (except the ENTRY node itself).
            for node in graph
                .externals(Direction::Incoming)
                .filter(|node| node != &entry)
                // Need to collect, otherwise there would be a lifetime issue.
                .collect::<Vec<_>>()
            {
                graph.add_edge(entry, node, ());
            }

            result.insert(*func_name, graph);
        }

        Ok(Cfgs(result))
    }
}

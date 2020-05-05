use std::collections::HashMap;

use petgraph::graph::DiGraph;
use petgraph::Direction;

use super::{Query, QueryInitError};
use crate::api::Api;
use crate::arena::{Arena, Dummy, P, S};
use crate::data::{statement::Statement, types::FuncName, RawData};

// Ideally a public constant, but we cannot make Arena::dummy a const fn because
// it is implemented in trait-bounded impl.
pub fn entry() -> P<Statement> {
    Arena::dummy(Dummy::D1)
}

// Ideally a public constant, but we cannot make Arena::dummy a const fn because
// it is implemented in trait-bounded impl.
pub fn exit() -> P<Statement> {
    Arena::dummy(Dummy::D2)
}

pub type Cfg = DiGraph<P<Statement>, ()>;

impl Query for Cfg {
    type Error = QueryInitError;
    type Args = S<FuncName>;

    fn init(data: &RawData, args: &Self::Args, _api: &Api) -> Result<Self, Self::Error> {
        data.modules
            .functions
            .iter()
            .find(|(func_name, _)| *func_name == args)
            .ok_or(QueryInitError::InvalidFuncName(*args))
            .map(|(_, func_body)| {
                let mut graph = DiGraph::with_capacity(func_body.len() + 1, func_body.len() + 1);
                let mut id_map = HashMap::new();

                for (id, stmt) in func_body.iter() {
                    id_map.insert(id, graph.add_node(stmt.clone()));
                }

                let entry = graph.add_node(entry());
                let exit = graph.add_node(exit());

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

                graph
            })
    }
}

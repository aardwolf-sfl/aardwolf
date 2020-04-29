use std::collections::HashMap;

use petgraph::graph::DiGraph;
use petgraph::Direction;

use crate::api::Api;
use crate::data::{
    statement::Statement,
    types::{FuncName, StmtId},
    RawData,
};
use crate::structures::{FromRawData, FromRawDataError};

pub const ENTRY: &'static Statement = &Statement::dummy(StmtId::dummy(std::u64::MAX - 1));
pub const EXIT: &'static Statement = &Statement::dummy(StmtId::dummy(std::u64::MAX));

pub type Cfg<'data> = DiGraph<&'data Statement, ()>;

pub struct Cfgs<'data>(HashMap<FuncName, Cfg<'data>>);

impl<'data> Cfgs<'data> {
    pub fn get(&'data self, func: &FuncName) -> Option<&'data Cfg<'data>> {
        self.0.get(func)
    }
}

impl<'data> FromRawData<'data> for Cfgs<'data> {
    fn from_raw(data: &'data RawData, _api: &'data Api<'data>) -> Result<Self, FromRawDataError> {
        let mut result = HashMap::new();

        for (func_name, func_body) in data.modules.functions.iter() {
            let mut graph = DiGraph::with_capacity(func_body.len() + 1, func_body.len() + 1);
            let mut id_map = HashMap::new();

            for (id, stmt) in func_body.iter() {
                id_map.insert(id, graph.add_node(stmt));
            }

            let entry = graph.add_node(ENTRY);
            let exit = graph.add_node(EXIT);

            for (id, stmt) in func_body.iter() {
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

            result.insert(func_name.clone(), graph);
        }

        Ok(Cfgs(result))
    }
}

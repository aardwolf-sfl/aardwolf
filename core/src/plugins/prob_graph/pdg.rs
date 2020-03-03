use std::collections::{HashMap, HashSet};

use petgraph::algo::dominators;
use petgraph::graph::{DiGraph, IndexType, NodeIndex};

use super::graph_ext::DominatorsExt;
use crate::raw::data::Statement;
use crate::structures::{Cfg, EXIT};

#[derive(Clone, Copy, PartialEq, Debug)]
enum EdgeTypePriv {
    ControlFlow,
    ControlDep,
    DataDep,
}

#[derive(Clone, Copy, Debug)]
pub enum EdgeType {
    ControlDep,
    DataDep,
}

type DataContext<T> = HashMap<u64, HashSet<T>>;

struct NodeData<'data, Ix> {
    stmt: &'data Statement,
    index: NodeIndex<Ix>,
    data_ctx: DataContext<NodeIndex<Ix>>,
    deps: HashSet<(u64, NodeIndex<Ix>)>,
}

impl<'data, Ix: IndexType> NodeData<'data, Ix> {
    pub fn new(stmt: &'data Statement, index: NodeIndex<Ix>) -> Self {
        NodeData {
            stmt,
            index,
            data_ctx: HashMap::new(),
            deps: HashSet::new(),
        }
    }

    pub fn prepare_update(&self, predecessor: &Self) -> Option<DataContext<NodeIndex<Ix>>> {
        let mut result = DataContext::new();

        for (var, pred_defs) in predecessor.data_ctx.iter() {
            let (defs, insert) = match self.data_ctx.get(var) {
                Some(node_defs) => {
                    let defs = node_defs.union(pred_defs).copied().collect::<HashSet<_>>();

                    if defs.len() > node_defs.len() {
                        // A new definition was added, update is necessary.
                        (defs, true)
                    } else {
                        // No new definition was added, no update is necessary.
                        (defs, false)
                    }
                }
                // An unknown variable, update is necessary.
                None => (pred_defs.iter().copied().collect(), true),
            };

            if insert {
                result.insert(*var, defs);
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    pub fn update(&mut self, data_ctx: DataContext<NodeIndex<Ix>>) {
        for (var, defs) in data_ctx {
            self.data_ctx.insert(var, defs);
        }
    }

    pub fn process(&mut self) {
        // Record data dependences of variables that this statements uses from current data context.
        // It's important to run this before changing the context with statement's defined variables
        // because of statements that use the variables that they also define.

        let vars = self
            .stmt
            .uses
            .iter()
            .flat_map(|access| access.get_scalars_for_use());

        for var in vars {
            if let Some(defs) = self.data_ctx.get(&var).map(|defs| defs.iter().copied()) {
                for def in defs {
                    self.deps.insert((var, def));
                }
            }
        }

        // For all variables that this statement defines, replace the current definition with this statement.
        let vars = self
            .stmt
            .defs
            .iter()
            .flat_map(|access| access.get_scalars_for_def());

        for var in vars {
            let defs = self.data_ctx.entry(var).or_insert(HashSet::new());
            defs.clear();
            defs.insert(self.index);
        }
    }

    pub fn dependences(&self) -> impl Iterator<Item = &(u64, NodeIndex<Ix>)> {
        self.deps.iter()
    }

    pub fn as_stmt(&self) -> &'data Statement {
        self.stmt
    }
}

pub type Pdg<'data> = DiGraph<&'data Statement, EdgeType>;

pub fn create_pdg<'data>(cfg: &Cfg<'data>) -> Pdg<'data> {
    let mut pdg = cfg.map(
        |index, stmt| NodeData::new(stmt, index),
        |_, _| EdgeTypePriv::ControlFlow,
    );

    compute_control_deps(&mut pdg, cfg);
    compute_data_deps(&mut pdg, cfg);

    // Remove control flow edges.
    pdg.retain_edges(|pdg, index| pdg[index] != EdgeTypePriv::ControlFlow);

    pdg.map(
        |_, node| node.as_stmt(),
        |_, edge| match edge {
            EdgeTypePriv::ControlFlow => unreachable!(),
            EdgeTypePriv::ControlDep => EdgeType::ControlDep,
            EdgeTypePriv::DataDep => EdgeType::DataDep,
        },
    )
}

fn compute_control_deps<'data, Ix: IndexType, E>(
    pdg: &mut DiGraph<NodeData<'data, Ix>, EdgeTypePriv, Ix>,
    cfg: &DiGraph<&'data Statement, E, Ix>,
) {
    // Reverse control flow edges so we compute post-dominance instead of dominance using the standard algorithm.
    pdg.reverse();

    let exit = cfg
        .node_indices()
        .find(|index| cfg[*index] == EXIT)
        .unwrap();

    // So far, there are only control flow edges,
    // so it is the same as if we run the algorithm on CFG itself.
    let dom = dominators::simple_fast(&*pdg, exit);
    let all_frontiers = dom.dominance_frontiers(&*pdg);

    for (node, frontiers) in all_frontiers {
        for dependence in frontiers {
            // Add the edge reversed, since we call pdg.reverse() later
            // to get control flow edges to original directions.
            pdg.add_edge(node, dependence, EdgeTypePriv::ControlDep);
        }
    }

    // Reverse control flow edges back.
    pdg.reverse();
}

fn compute_data_deps<'data, Ix: IndexType, E>(
    pdg: &mut DiGraph<NodeData<'data, Ix>, EdgeTypePriv, Ix>,
    cfg: &DiGraph<&'data Statement, E, Ix>,
) {
    let mut queue = cfg.node_indices().collect::<Vec<_>>();

    // While there are some nodes that still need to be processed.
    while let Some(current) = queue.pop() {
        // Process the current context in the node.
        pdg[current].process();

        // Iterate over successors in original CFG.
        for succ in cfg.neighbors(current) {
            if let Some(data_ctx) = pdg[succ].prepare_update(&pdg[current]) {
                pdg[succ].update(data_ctx);
                // If the successor was updated, we need to process it again later with new information.
                queue.push(succ);
            }
        }
    }

    let edges = pdg
        .node_indices()
        .flat_map(|index| {
            pdg[index]
                .dependences()
                .map(move |(var, dependence)| (*dependence, *var, index))
        })
        .collect::<Vec<_>>();

    for (source, _, target) in edges {
        pdg.add_edge(source, target, EdgeTypePriv::DataDep);
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use petgraph::algo;
    use petgraph::graph::DiGraph;

    use crate::raw::data::{Access, Statement};
    use crate::structures::{ENTRY, EXIT};

    pub struct StatementFactory(HashMap<u64, Statement>);

    impl StatementFactory {
        pub fn new() -> Self {
            StatementFactory(HashMap::new())
        }

        pub fn add(&mut self, id: u64) {
            self.0.insert(id, Statement::dummy(id));
        }

        pub fn add_many(&mut self, ids: impl Iterator<Item = u64>) {
            for id in ids {
                self.add(id);
            }
        }

        pub fn add_def(&mut self, id: u64, access: Access) {
            if let Some(stmt) = self.0.get_mut(&id) {
                // Valid because we don't modify statement id
                // which is the only field used to compute the hash.
                stmt.defs.push(access);
            }
        }

        pub fn add_use(&mut self, id: u64, access: Access) {
            if let Some(stmt) = self.0.get_mut(&id) {
                // Valid because we don't modify statement id
                // which is the only field used to compute the hash.
                stmt.uses.push(access);
            }
        }

        pub fn add_succ(&mut self, id: u64, succ: u64) {
            if let Some(stmt) = self.0.get_mut(&id) {
                // Valid because we don't modify statement id
                // which is the only field used to compute the hash.
                stmt.succ.push(succ);
            }
        }

        pub fn get(&self, id: u64) -> &Statement {
            // This structure is for testing purposes, safe API (returning Option) is not needed.
            &self.0[&id]
        }
    }

    pub fn create_basic_cfg(factory: &mut StatementFactory) -> Cfg {
        // Example program from "The probabilistic program dependence graph and its application to fault diagnosis"
        let mut cfg = DiGraph::new();

        factory.add_many(1..=10);

        let var_i = 1;
        let var_n = 2;
        let var_max = 3;
        let var_v = 4;

        factory.add_def(1, Access::Scalar(var_i));
        factory.add_def(2, Access::Scalar(var_n));
        factory.add_def(3, Access::Scalar(var_max));
        factory.add_def(5, Access::Scalar(var_v));
        factory.add_def(7, Access::Scalar(var_max));
        factory.add_def(8, Access::Scalar(var_i));

        factory.add_use(4, Access::Scalar(var_i));
        factory.add_use(4, Access::Scalar(var_n));
        factory.add_use(6, Access::Scalar(var_v));
        factory.add_use(6, Access::Scalar(var_max));
        factory.add_use(7, Access::Scalar(var_v));
        factory.add_use(8, Access::Scalar(var_i));
        factory.add_use(4, Access::Scalar(var_i));
        factory.add_use(10, Access::Scalar(var_max));

        // Add only successors of predicates which is needed for is_predicate method of the statement.
        factory.add_succ(4, 5);
        factory.add_succ(4, 10);
        factory.add_succ(6, 7);
        factory.add_succ(6, 8);

        let entry = cfg.add_node(ENTRY);
        let n1 = cfg.add_node(factory.get(1));
        let n2 = cfg.add_node(factory.get(2));
        let n3 = cfg.add_node(factory.get(3));
        let n4 = cfg.add_node(factory.get(4));
        let n5 = cfg.add_node(factory.get(5));
        let n6 = cfg.add_node(factory.get(6));
        let n7 = cfg.add_node(factory.get(7));
        let n8 = cfg.add_node(factory.get(8));
        let n10 = cfg.add_node(factory.get(10));
        let exit = cfg.add_node(EXIT);

        cfg.add_edge(entry, n1, ());
        cfg.add_edge(n1, n2, ());
        cfg.add_edge(n2, n3, ());
        cfg.add_edge(n3, n4, ());
        cfg.add_edge(n4, n5, ());
        cfg.add_edge(n5, n6, ());
        cfg.add_edge(n6, n7, ());
        cfg.add_edge(n7, n8, ());
        cfg.add_edge(n6, n8, ());
        cfg.add_edge(n8, n4, ());
        cfg.add_edge(n4, n10, ());
        cfg.add_edge(n10, exit, ());

        cfg
    }

    #[test]
    fn basic() {
        let mut factory = StatementFactory::new();
        let cfg = create_basic_cfg(&mut factory);

        let actual = create_pdg(&cfg);

        let mut factory = StatementFactory::new();
        factory.add_many(1..=10);

        let mut expected = DiGraph::new();

        let _ = expected.add_node(ENTRY);
        let n1 = expected.add_node(factory.get(1));
        let n2 = expected.add_node(factory.get(2));
        let n3 = expected.add_node(factory.get(3));
        let n4 = expected.add_node(factory.get(4));
        let n5 = expected.add_node(factory.get(5));
        let n6 = expected.add_node(factory.get(6));
        let n7 = expected.add_node(factory.get(7));
        let n8 = expected.add_node(factory.get(8));
        let n10 = expected.add_node(factory.get(10));
        let _ = expected.add_node(EXIT);

        expected.add_edge(n1, n4, EdgeType::DataDep);
        expected.add_edge(n1, n8, EdgeType::DataDep);
        expected.add_edge(n2, n4, EdgeType::DataDep);
        expected.add_edge(n3, n6, EdgeType::DataDep);
        expected.add_edge(n3, n10, EdgeType::DataDep);
        expected.add_edge(n4, n4, EdgeType::ControlDep);
        expected.add_edge(n4, n5, EdgeType::ControlDep);
        expected.add_edge(n4, n6, EdgeType::ControlDep);
        expected.add_edge(n4, n8, EdgeType::ControlDep);
        expected.add_edge(n5, n6, EdgeType::DataDep);
        expected.add_edge(n5, n7, EdgeType::DataDep);
        expected.add_edge(n6, n7, EdgeType::ControlDep);
        expected.add_edge(n7, n6, EdgeType::DataDep);
        expected.add_edge(n7, n10, EdgeType::DataDep);
        expected.add_edge(n8, n4, EdgeType::DataDep);
        expected.add_edge(n8, n8, EdgeType::DataDep);

        assert!(
            algo::is_isomorphic(&expected, &actual),
            "Graphs are not isomorphic"
        );
    }
}

use std::collections::{HashMap, HashSet};

use petgraph::algo::dominators;
use petgraph::graph::{DiGraph, IndexType, NodeIndex};

use crate::api::Api;
use crate::arena::{P, S};
use crate::data::{access::AccessChain, statement::Statement, types::FuncName, RawData};
use crate::graph_ext::DominatorsExt;
use crate::structures::{Cfg, Cfgs, FromRawData, FromRawDataError};

pub type Pdg = DiGraph<P<Statement>, EdgeType>;

pub struct Pdgs(HashMap<S<FuncName>, Pdg>);

impl Pdgs {
    pub fn get(&self, func: &S<FuncName>) -> Option<&Pdg> {
        self.0.get(func)
    }
}

impl FromRawData for Pdgs {
    fn from_raw(data: &RawData, api: &Api) -> Result<Self, FromRawDataError> {
        let mut result = HashMap::new();
        let cfgs = api.get_cfgs();

        for (func_name, _) in data.modules.functions.iter() {
            result.insert(*func_name, create_pdg(cfgs.get(func_name).unwrap()));
        }

        Ok(Pdgs(result))
    }
}

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

struct NodeData<Ix> {
    stmt: P<Statement>,
    index: NodeIndex<Ix>,
    data_ctx: DataContext<NodeIndex<Ix>>,
    deps: HashSet<(u64, NodeIndex<Ix>)>,
}

impl<Ix: IndexType> NodeData<Ix> {
    pub fn new(stmt: P<Statement>, index: NodeIndex<Ix>) -> Self {
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
            .as_ref()
            .uses
            .iter()
            .map(|access| access.as_ref())
            .flat_map(AccessChain::from_uses);

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
            .as_ref()
            .defs
            .iter()
            .map(|access| access.as_ref())
            .flat_map(AccessChain::from_defs);

        for var in vars {
            let defs = self.data_ctx.entry(var).or_insert(HashSet::new());
            defs.clear();
            defs.insert(self.index);
        }
    }

    pub fn dependences(&self) -> impl Iterator<Item = &(u64, NodeIndex<Ix>)> {
        self.deps.iter()
    }

    pub fn as_stmt(&self) -> P<Statement> {
        self.stmt
    }
}

pub fn create_pdg(cfg: &Cfg) -> Pdg {
    let mut pdg = cfg.map(
        |index, stmt| NodeData::new(*stmt, index),
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

fn compute_control_deps<Ix: IndexType, E>(
    pdg: &mut DiGraph<NodeData<Ix>, EdgeTypePriv, Ix>,
    cfg: &DiGraph<P<Statement>, E, Ix>,
) {
    // Reverse control flow edges so we compute post-dominance instead of dominance using the standard algorithm.
    pdg.reverse();

    let exit = cfg
        .node_indices()
        .find(|index| cfg[*index] == Cfgs::exit())
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

fn compute_data_deps<Ix: IndexType, E>(
    pdg: &mut DiGraph<NodeData<Ix>, EdgeTypePriv, Ix>,
    cfg: &DiGraph<P<Statement>, E, Ix>,
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

    use crate::arena::{Arena, P};
    use crate::data::{access::Access, statement::Statement, types::StmtId};
    use crate::structures::Cfgs;

    pub struct StatementFactory {
        data: HashMap<StmtId, P<Statement>>,
        stmts: Arena<Statement>,
        accesses: Arena<Access>,
    }

    impl StatementFactory {
        pub fn new() -> Self {
            StatementFactory {
                data: HashMap::new(),
                stmts: Arena::with_capacity(32),
                accesses: Arena::with_capacity(32),
            }
        }

        pub fn add(&mut self, id: StmtId) {
            let ptr = self.stmts.alloc(Statement::new_test(id));
            self.data.insert(id, ptr);
        }

        pub fn add_many(&mut self, ids: impl Iterator<Item = StmtId>) {
            for id in ids {
                self.add(id);
            }
        }

        pub fn create_access(&mut self, access: Access) -> P<Access> {
            self.accesses.alloc(access)
        }

        pub fn add_def(&mut self, id: StmtId, access: P<Access>) {
            if let Some(stmt) = self.data.get(&id) {
                // Valid because we don't modify statement id
                // which is the only field used to compute the hash.
                self.stmts.get_mut(stmt).defs.push(access);
            }
        }

        pub fn add_use(&mut self, id: StmtId, access: P<Access>) {
            if let Some(stmt) = self.data.get(&id) {
                // Valid because we don't modify statement id
                // which is the only field used to compute the hash.
                self.stmts.get_mut(stmt).uses.push(access);
            }
        }

        pub fn add_succ(&mut self, id: StmtId, succ: StmtId) {
            if let Some(stmt) = self.data.get(&id) {
                // Valid because we don't modify statement id
                // which is the only field used to compute the hash.
                self.stmts.get_mut(stmt).succ.push(succ);
            }
        }

        pub fn seal(self) -> SealedStatementFactory {
            P::<Statement>::init_once(self.stmts);
            P::<Access>::init_once(self.accesses);
            SealedStatementFactory(self.data)
        }
    }

    pub struct SealedStatementFactory(HashMap<StmtId, P<Statement>>);

    impl SealedStatementFactory {
        pub fn get(&self, id: StmtId) -> P<Statement> {
            // This structure is for testing purposes, safe API (returning Option) is not needed.
            self.0[&id]
        }
    }

    pub fn create_basic_cfg() -> (Cfg, SealedStatementFactory) {
        // Example program from "The probabilistic program dependence graph and its application to fault diagnosis"
        let mut cfg = DiGraph::new();

        let mut factory = StatementFactory::new();

        factory.add_many((1..=10).map(|stmt_id| StmtId::new_test(stmt_id)));

        let var_i = factory.create_access(Access::Scalar(1));
        let var_n = factory.create_access(Access::Scalar(2));
        let var_max = factory.create_access(Access::Scalar(3));
        let var_v = factory.create_access(Access::Scalar(4));

        factory.add_def(StmtId::new_test(1), var_i);
        factory.add_def(StmtId::new_test(2), var_n);
        factory.add_def(StmtId::new_test(3), var_max);
        factory.add_def(StmtId::new_test(5), var_v);
        factory.add_def(StmtId::new_test(7), var_max);
        factory.add_def(StmtId::new_test(8), var_i);

        factory.add_use(StmtId::new_test(4), var_i);
        factory.add_use(StmtId::new_test(4), var_n);
        factory.add_use(StmtId::new_test(6), var_v);
        factory.add_use(StmtId::new_test(6), var_max);
        factory.add_use(StmtId::new_test(7), var_v);
        factory.add_use(StmtId::new_test(8), var_i);
        factory.add_use(StmtId::new_test(4), var_i);
        factory.add_use(StmtId::new_test(10), var_max);

        // Add only successors of predicates which is needed for is_predicate method of the statement.
        factory.add_succ(StmtId::new_test(4), StmtId::new_test(5));
        factory.add_succ(StmtId::new_test(4), StmtId::new_test(10));
        factory.add_succ(StmtId::new_test(6), StmtId::new_test(7));
        factory.add_succ(StmtId::new_test(6), StmtId::new_test(8));

        let factory = factory.seal();

        let entry = cfg.add_node(Cfgs::entry());
        let n1 = cfg.add_node(factory.get(StmtId::new_test(1)));
        let n2 = cfg.add_node(factory.get(StmtId::new_test(2)));
        let n3 = cfg.add_node(factory.get(StmtId::new_test(3)));
        let n4 = cfg.add_node(factory.get(StmtId::new_test(4)));
        let n5 = cfg.add_node(factory.get(StmtId::new_test(5)));
        let n6 = cfg.add_node(factory.get(StmtId::new_test(6)));
        let n7 = cfg.add_node(factory.get(StmtId::new_test(7)));
        let n8 = cfg.add_node(factory.get(StmtId::new_test(8)));
        let n10 = cfg.add_node(factory.get(StmtId::new_test(10)));
        let exit = cfg.add_node(Cfgs::exit());

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

        (cfg, factory)
    }

    #[test]
    fn basic() {
        let (cfg, factory) = create_basic_cfg();

        let actual = create_pdg(&cfg);

        let mut expected = DiGraph::new();

        let _ = expected.add_node(Cfgs::entry());
        let n1 = expected.add_node(factory.get(StmtId::new_test(1)));
        let n2 = expected.add_node(factory.get(StmtId::new_test(2)));
        let n3 = expected.add_node(factory.get(StmtId::new_test(3)));
        let n4 = expected.add_node(factory.get(StmtId::new_test(4)));
        let n5 = expected.add_node(factory.get(StmtId::new_test(5)));
        let n6 = expected.add_node(factory.get(StmtId::new_test(6)));
        let n7 = expected.add_node(factory.get(StmtId::new_test(7)));
        let n8 = expected.add_node(factory.get(StmtId::new_test(8)));
        let n10 = expected.add_node(factory.get(StmtId::new_test(10)));
        let _ = expected.add_node(Cfgs::exit());

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

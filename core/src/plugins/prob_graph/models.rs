use std::collections::HashMap;
use std::ops::Index;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::api::Api;
use crate::arena::P;
use crate::data::statement::Statement;
use crate::plugins::prob_graph::{trace::Trace, Ppdg};
use crate::plugins::{LocalizationItem, PluginError, Rationale, Results};
use crate::structures::{EdgeType as PdgEdgeType, Pdg};

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum NodeType {
    SelfLoop,
    NonPredicate,
    Predicate,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EdgeType {
    ControlDep,
    DataDep,
    StateSplit,
}

impl From<PdgEdgeType> for EdgeType {
    fn from(value: PdgEdgeType) -> Self {
        match value {
            PdgEdgeType::ControlDep => EdgeType::ControlDep,
            PdgEdgeType::DataDep => EdgeType::DataDep,
        }
    }
}

impl EdgeType {
    pub fn is_control_dep(&self) -> bool {
        self == &EdgeType::ControlDep
    }

    pub fn is_data_dep(&self) -> bool {
        self == &EdgeType::DataDep
    }

    pub fn is_state_split(&self) -> bool {
        self == &EdgeType::StateSplit
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    pub stmt: P<Statement>,
    pub typ: NodeType,
}

impl Node {
    pub fn new(stmt: P<Statement>, typ: NodeType) -> Self {
        Node { stmt, typ }
    }

    pub fn to_predicate(self) -> Self {
        Node {
            typ: NodeType::Predicate,
            ..self
        }
    }

    pub fn to_non_predicate(self) -> Self {
        Node {
            typ: NodeType::NonPredicate,
            ..self
        }
    }

    pub fn to_self_loop(self) -> Self {
        Node {
            typ: NodeType::SelfLoop,
            ..self
        }
    }
}

pub trait Model {
    fn get_graph(&self) -> &ModelGraph;
    fn from_pdg(pdg: &Pdg) -> Self;
    fn run_loc<'param, I: Iterator<Item = P<Statement>>>(
        trace: Trace<I, Self>,
        ppdg: &Ppdg,
        api: &Api,
        results: &'param mut Results,
    ) -> Result<(), PluginError>
    where
        Self: Sized;
}

pub enum StmtNodes {
    Just([NodeIndex; 1]),
    Split([NodeIndex; 2]),
}

impl StmtNodes {
    pub fn new(index: NodeIndex) -> Self {
        StmtNodes::Just([index])
    }

    pub fn add(self, index: NodeIndex) -> Self {
        // Either node-splitting or self-loop elimination. In both cases, new index is predecessor.
        match self {
            StmtNodes::Just([original]) => StmtNodes::Split([index, original]),
            StmtNodes::Split([_, original]) => {
                debug_assert!(false, "cannot add new index when the node is already split");
                StmtNodes::Split([index, original])
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &NodeIndex> {
        match self {
            StmtNodes::Just(just) => just.iter(),
            StmtNodes::Split(split) => split.iter(),
        }
    }
}

pub struct ModelGraph {
    graph: DiGraph<Node, EdgeType>,
    mapping: HashMap<P<Statement>, StmtNodes>,
}

impl ModelGraph {
    pub fn new(graph: DiGraph<Node, EdgeType>, mapping: HashMap<P<Statement>, StmtNodes>) -> Self {
        ModelGraph { graph, mapping }
    }
}

impl AsRef<DiGraph<Node, EdgeType>> for ModelGraph {
    fn as_ref(&self) -> &DiGraph<Node, EdgeType> {
        &self.graph
    }
}

impl AsMut<DiGraph<Node, EdgeType>> for ModelGraph {
    fn as_mut(&mut self) -> &mut DiGraph<Node, EdgeType> {
        &mut self.graph
    }
}

impl Index<&P<Statement>> for ModelGraph {
    type Output = StmtNodes;

    fn index(&self, stmt: &P<Statement>) -> &Self::Output {
        &self.mapping[stmt]
    }
}

impl Index<NodeIndex> for ModelGraph {
    type Output = Node;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.graph[index]
    }
}

pub struct DependencyNetwork(ModelGraph);

impl Model for DependencyNetwork {
    fn get_graph(&self) -> &ModelGraph {
        &self.0
    }

    fn from_pdg(pdg: &Pdg) -> Self {
        let (graph, mapping) = create_dependency_network(pdg);
        DependencyNetwork(ModelGraph::new(graph, mapping))
    }

    fn run_loc<'param, I: Iterator<Item = P<Statement>>>(
        trace: Trace<I, Self>,
        ppdg: &Ppdg,
        _api: &Api,
        results: &'param mut Results,
    ) -> Result<(), PluginError>
    where
        Self: Sized,
    {
        let mut probs = HashMap::new();

        for (index, item) in trace.enumerate() {
            let prob = ppdg.get_prob(&item);

            let lowest_prob = probs
                .get(&item.node.stmt)
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

        let mut probs = probs.into_iter().collect::<Vec<_>>();

        // Sort the probs by index. If there are some ties in score,
        // this will prioritizes statements that occur earlier.
        probs.sort_unstable_by(|lhs, rhs| (lhs.1).1.cmp(&(rhs.1).1));

        for (stmt, (prob, _)) in probs {
            results.add(
                LocalizationItem::new(
                    stmt.as_ref().loc,
                    stmt,
                    1.0 - prob,
                    default_rationale.clone(),
                )
                .unwrap(),
            );
        }

        Ok(())
    }
}

pub struct BayesianNetwork(ModelGraph);

impl Model for BayesianNetwork {
    fn get_graph(&self) -> &ModelGraph {
        &self.0
    }

    fn from_pdg(pdg: &Pdg) -> Self {
        let (graph, mapping) = create_bayesian_network(pdg);
        BayesianNetwork(ModelGraph::new(graph, mapping))
    }

    fn run_loc<'param, I: Iterator<Item = P<Statement>>>(
        _trace: Trace<I, Self>,
        _ppdg: &Ppdg,
        _api: &Api,
        _results: &'param mut Results,
    ) -> Result<(), PluginError>
    where
        Self: Sized,
    {
        // TODO

        Ok(())
    }
}

fn create_dependency_network(
    pdg: &Pdg,
) -> (DiGraph<Node, EdgeType>, HashMap<P<Statement>, StmtNodes>) {
    let mut dn = pdg.map(
        |_, node| {
            Node::new(
                *node,
                if node.as_ref().is_predicate() {
                    NodeType::Predicate
                } else {
                    NodeType::NonPredicate
                },
            )
        },
        |_, edge| EdgeType::from(*edge),
    );

    let mut mapping = HashMap::new();

    let mut remove = Vec::new();

    for index in dn.node_indices() {
        let mut stmt_nodes = StmtNodes::new(index);

        // Split two-state (predicate and data) nodes.
        let has_predicate_state = dn
            .edges_directed(index, Direction::Outgoing)
            .any(|edge| edge.weight().is_control_dep());
        let has_data_state = dn
            .edges_directed(index, Direction::Incoming)
            .any(|edge| edge.weight().is_data_dep());

        if has_predicate_state && has_data_state {
            let data_index = dn.add_node(dn[index].to_non_predicate());

            let incoming = dn
                .edges_directed(index, Direction::Incoming)
                .map(|edge| (edge.id(), edge.source()))
                .collect::<Vec<_>>();

            // This also handles self-loops correctly.
            for (edge_index, source) in incoming {
                // TODO: Use in-place endpoint modifications when it is implemented in petgraph.
                //       Tracking issue: https://github.com/petgraph/petgraph/issues/333
                // dn.update_target(edge_index, data_index);
                remove.push(edge_index);
                dn.add_edge(source, data_index, dn[edge_index]);
            }

            dn.add_edge(data_index, index, EdgeType::StateSplit);

            stmt_nodes = stmt_nodes.add(data_index);
        } else if let Some(edge_index) = dn.find_edge(index, index) {
            // Remove self-loops of nodes which were not handled during node splitting.
            let self_loop_node = dn.add_node(dn[index].to_self_loop());
            // TODO: Use in-place endpoint modifications when it is implemented in petgraph.
            //       Tracking issue: https://github.com/petgraph/petgraph/issues/333
            // dn.update_source(edge_index, self_loop_node);
            remove.push(edge_index);
            dn.add_edge(self_loop_node, index, dn[edge_index]);

            stmt_nodes = stmt_nodes.add(self_loop_node);
        }

        mapping.insert(dn[index].stmt, stmt_nodes);
    }

    for edge_index in remove {
        dn.remove_edge(edge_index);
    }

    (dn, mapping)
}

pub fn create_bayesian_network(
    pdg: &Pdg,
) -> (DiGraph<Node, EdgeType>, HashMap<P<Statement>, StmtNodes>) {
    let (dn, mapping) = create_dependency_network(pdg);

    // TODO: Transform to Bayesian network.

    (dn, mapping)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structures::pdg::{create_pdg, tests::*};

    use petgraph::algo;
    use petgraph::graph::DiGraph;

    use crate::data::types::StmtId;
    use crate::structures::Cfgs;

    #[test]
    fn dependency_network_basic() {
        let (cfg, factory) = create_basic_cfg();

        let pdg = create_pdg(&cfg);
        let (actual, _) = create_dependency_network(&pdg);

        let mut expected = DiGraph::new();

        let _ = expected.add_node(Node::new(Cfgs::entry(), NodeType::NonPredicate));
        let n1 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(1)),
            NodeType::NonPredicate,
        ));
        let n2 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(2)),
            NodeType::NonPredicate,
        ));
        let n3 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(3)),
            NodeType::NonPredicate,
        ));
        let n4 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(4)),
            NodeType::Predicate,
        ));
        let n4_data = expected.add_node(Node::new(
            factory.get(StmtId::new_test(4)),
            NodeType::NonPredicate,
        ));
        let n5 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(5)),
            NodeType::NonPredicate,
        ));
        let n6 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(6)),
            NodeType::Predicate,
        ));
        let n6_data = expected.add_node(Node::new(
            factory.get(StmtId::new_test(6)),
            NodeType::NonPredicate,
        ));
        let n7 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(7)),
            NodeType::NonPredicate,
        ));
        let n8 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(8)),
            NodeType::NonPredicate,
        ));
        let n8_loop = expected.add_node(Node::new(
            factory.get(StmtId::new_test(8)),
            NodeType::SelfLoop,
        ));
        let n10 = expected.add_node(Node::new(
            factory.get(StmtId::new_test(10)),
            NodeType::NonPredicate,
        ));
        let _ = expected.add_node(Node::new(Cfgs::exit(), NodeType::NonPredicate));

        expected.add_edge(n1, n4_data, EdgeType::DataDep);
        expected.add_edge(n1, n8, EdgeType::DataDep);
        expected.add_edge(n2, n4_data, EdgeType::DataDep);
        expected.add_edge(n3, n6_data, EdgeType::DataDep);
        expected.add_edge(n3, n10, EdgeType::DataDep);
        expected.add_edge(n4_data, n4, EdgeType::StateSplit);
        expected.add_edge(n4, n4_data, EdgeType::ControlDep);
        expected.add_edge(n4, n5, EdgeType::ControlDep);
        expected.add_edge(n4, n6_data, EdgeType::ControlDep);
        expected.add_edge(n4, n8, EdgeType::ControlDep);
        expected.add_edge(n5, n6_data, EdgeType::DataDep);
        expected.add_edge(n5, n7, EdgeType::DataDep);
        expected.add_edge(n6_data, n6, EdgeType::StateSplit);
        expected.add_edge(n6, n7, EdgeType::ControlDep);
        expected.add_edge(n7, n6_data, EdgeType::DataDep);
        expected.add_edge(n7, n10, EdgeType::DataDep);
        expected.add_edge(n8, n4_data, EdgeType::DataDep);
        expected.add_edge(n8_loop, n8, EdgeType::DataDep);

        assert!(
            algo::is_isomorphic(&expected, &actual),
            "Graphs are not isomorphic"
        );
    }
}

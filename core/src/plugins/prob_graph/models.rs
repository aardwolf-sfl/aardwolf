use std::collections::{HashMap, HashSet};

use petgraph::graph::{DiGraph, IndexType, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use super::pdg;
use crate::raw::data::Statement;

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum NodeType {
    // Numbers determine the ordering.
    SelfLoop = 1,
    NonPredicate = 2,
    Predicate = 3,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EdgeType {
    ControlDep,
    DataDep,
    StateSplit,
}

impl From<pdg::EdgeType> for EdgeType {
    fn from(value: pdg::EdgeType) -> Self {
        match value {
            pdg::EdgeType::ControlDep => EdgeType::ControlDep,
            pdg::EdgeType::DataDep => EdgeType::DataDep,
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
pub struct Node<'a> {
    pub stmt: &'a Statement,
    pub typ: NodeType,
}

impl<'a> Node<'a> {
    pub fn new(stmt: &'a Statement, typ: NodeType) -> Self {
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

pub trait Model<'a> {
    fn get_graph(&self) -> &ModelGraph<'a>;
    fn from_pdg(pdg: &pdg::Pdg<'a>) -> Self;
}

pub type ModelGraph<'a> = DiGraph<Node<'a>, EdgeType>;

pub struct DependencyNetwork<'a>(ModelGraph<'a>);

impl<'a> Model<'a> for DependencyNetwork<'a> {
    fn get_graph(&self) -> &ModelGraph<'a> {
        &self.0
    }

    fn from_pdg(pdg: &pdg::Pdg<'a>) -> Self {
        DependencyNetwork(create_dependency_network(pdg))
    }
}

pub struct BayesianNetwork<'a>(ModelGraph<'a>);

impl<'a> Model<'a> for BayesianNetwork<'a> {
    fn get_graph(&self) -> &ModelGraph<'a> {
        &self.0
    }

    fn from_pdg(pdg: &pdg::Pdg<'a>) -> Self {
        BayesianNetwork(create_bayesian_network(pdg))
    }
}

fn create_dependency_network<'a>(pdg: &pdg::Pdg<'a>) -> ModelGraph<'a> {
    let mut dn = pdg.map(
        |_, node| {
            Node::new(
                node,
                if node.is_predicate() {
                    NodeType::Predicate
                } else {
                    NodeType::NonPredicate
                },
            )
        },
        |_, edge| EdgeType::from(*edge),
    );

    let mut remove = Vec::new();

    for index in dn.node_indices() {
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
        } else if let Some(edge_index) = dn.find_edge(index, index) {
            // Remove self-loops of nodes which were not handled during node splitting.
            let self_loop_node = dn.add_node(dn[index].to_self_loop());
            // TODO: Use in-place endpoint modifications when it is implemented in petgraph.
            //       Tracking issue: https://github.com/petgraph/petgraph/issues/333
            // dn.update_source(edge_index, self_loop_node);
            remove.push(edge_index);
            dn.add_edge(self_loop_node, index, dn[edge_index]);
        }
    }

    for edge_index in remove {
        dn.remove_edge(edge_index);
    }

    dn
}

pub fn create_bayesian_network<'a>(pdg: &pdg::Pdg<'a>) -> ModelGraph<'a> {
    let dn = create_dependency_network(pdg);

    // TODO: Transform to Bayesian network.

    dn
}

#[cfg(test)]
mod tests {
    use super::super::pdg::{create_pdg, tests::*};
    use super::*;

    use petgraph::algo;
    use petgraph::graph::DiGraph;

    use crate::raw::data::Statement;
    use crate::structures::{ENTRY, EXIT};

    #[test]
    fn dependency_network_basic() {
        let mut factory = StatementFactory::new();
        let cfg = create_basic_cfg(&mut factory);

        let pdg = create_pdg(&cfg);
        let actual = create_dependency_network(&pdg);

        let mut factory = StatementFactory::new();
        factory.add_many(1..=10);

        let mut expected = DiGraph::new();

        let entry = expected.add_node(Node::new(ENTRY, NodeType::NonPredicate));
        let n1 = expected.add_node(Node::new(factory.get(1), NodeType::NonPredicate));
        let n2 = expected.add_node(Node::new(factory.get(2), NodeType::NonPredicate));
        let n3 = expected.add_node(Node::new(factory.get(3), NodeType::NonPredicate));
        let n4 = expected.add_node(Node::new(factory.get(4), NodeType::Predicate));
        let n4_data = expected.add_node(Node::new(factory.get(4), NodeType::NonPredicate));
        let n5 = expected.add_node(Node::new(factory.get(5), NodeType::NonPredicate));
        let n6 = expected.add_node(Node::new(factory.get(6), NodeType::Predicate));
        let n6_data = expected.add_node(Node::new(factory.get(6), NodeType::NonPredicate));
        let n7 = expected.add_node(Node::new(factory.get(7), NodeType::NonPredicate));
        let n8 = expected.add_node(Node::new(factory.get(8), NodeType::NonPredicate));
        let n8_loop = expected.add_node(Node::new(factory.get(8), NodeType::SelfLoop));
        let n10 = expected.add_node(Node::new(factory.get(10), NodeType::NonPredicate));
        let exit = expected.add_node(Node::new(EXIT, NodeType::NonPredicate));

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
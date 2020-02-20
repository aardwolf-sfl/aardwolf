#![deny(warnings)]

use petgraph::algo::dominators::Dominators;
use petgraph::visit::{
    GraphBase, IntoNeighbors, IntoNeighborsDirected, IntoNodeIdentifiers, VisitMap, Visitable,
};
use petgraph::Direction;
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::FromIterator;

pub trait DominatorsExt<N>
where
    N: Copy + Eq + Hash,
{
    fn dominance_frontiers<G>(&self, graph: G) -> HashMap<G::NodeId, Vec<G::NodeId>>
    where
        <G as Visitable>::Map: VisitMap<N>,
        G: IntoNeighborsDirected
            + IntoNodeIdentifiers
            + IntoNeighbors
            + Visitable
            + GraphBase<NodeId = N>,
        <G as IntoNeighborsDirected>::NeighborsDirected: Clone,
        <G as GraphBase>::NodeId: Eq + Hash + Ord;
}

impl<N> DominatorsExt<N> for Dominators<N>
where
    N: Copy + Eq + Hash,
{
    // Copied from closed PR https://github.com/petgraph/petgraph/pull/178
    fn dominance_frontiers<G>(&self, graph: G) -> HashMap<G::NodeId, Vec<G::NodeId>>
    where
        <G as Visitable>::Map: VisitMap<N>,
        G: IntoNeighborsDirected
            + IntoNodeIdentifiers
            + IntoNeighbors
            + Visitable
            + GraphBase<NodeId = N>,
        <G as IntoNeighborsDirected>::NeighborsDirected: Clone,
        <G as GraphBase>::NodeId: Eq + Hash + Ord,
    {
        let mut frontiers = HashMap::<G::NodeId, Vec<G::NodeId>>::from_iter(
            graph.node_identifiers().map(|v| (v, vec![])),
        );

        for node in graph.node_identifiers() {
            let (predecessors, predecessors_len) = {
                let ret = graph.neighbors_directed(node, Direction::Incoming);
                let count = ret.clone().count();
                (ret, count)
            };

            if predecessors_len >= 2 {
                for p in predecessors {
                    let mut runner = p;

                    match self.immediate_dominator(node) {
                        Some(dominator) => {
                            while runner != dominator {
                                frontiers.entry(runner).or_insert(vec![]).push(node);
                                runner = self.immediate_dominator(runner).unwrap();
                            }
                        }
                        None => (),
                    }
                }
                for (_, frontier) in frontiers.iter_mut() {
                    frontier.sort();
                    frontier.dedup();
                }
            }
        }
        frontiers
    }
}

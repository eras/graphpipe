use crate::graph;
use fjadra::{Link, Node, Simulation, SimulationBuilder};
use petgraph::visit::IntoNodeReferences;
use petgraph::visit::EdgeRef;

#[derive(serde::Serialize, Debug, Clone)]
pub struct NodePos {
    node: graph::Node,
    pos: (f64, f64),
}

pub struct Layout {
    nodes: Vec<graph::Node>,
    edges: Vec<(graph::NodeId, graph::NodeId, graph::Edge)>,
    sim: Simulation,
}

impl From<graph::Node> for Node {
    fn from(node: graph::Node) -> Self {
        Node::default()
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct NodesEdges {
    nodes: Vec<NodePos>,
    edges: Vec<(graph::NodeId, graph::NodeId, graph::Edge)>,
}

impl Layout {
    pub fn new(g: &graph::Graph) -> Self {
        let edges = g.graph.edge_references();
        let nodes: Vec<graph::Node> = g
            .graph
            .node_references()
            .into_iter()
            .map(|(node_index, node)| node.clone())
            .collect();
        let sim = SimulationBuilder::default()
            .build(nodes.iter().map(|node| node.layout_node()))
            .add_force(
                "link",
                Link::new(edges.clone().into_iter().map(|edge| (0usize, 0usize)))
                    .strength(1.0)
                    .distance(60.0)
                    .iterations(10),
            );
        Layout {
            sim,
            nodes,
            edges: g
                .graph
                .edge_references()
                .map(|edge| {
                    (
                        g.resolve_node_id(edge.source()),
                        g.resolve_node_id(edge.target()),
                        edge.weight().clone(),
                    )
                })
                .collect(),
        }
    }

    pub fn step(&mut self) -> NodesEdges {
        self.sim.step();
        let nodes =
            std::iter::zip(self.nodes.iter(), self.sim.positions()).map(|(node, pos)| NodePos {
                node: node.clone(),
                pos: (pos[0] * 10.0, pos[1] * 10.0),
            });
        NodesEdges {
            nodes: nodes.collect(),
            edges: self.edges.clone(),
        }
    }
}

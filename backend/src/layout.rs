use crate::graph;
use fjadra::{Link, Node, Simulation, SimulationBuilder, ManyBody};
use petgraph::visit::IntoNodeReferences;
use petgraph::visit::EdgeRef;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    GraphError(#[from] crate::graph::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(serde::Serialize, Debug, Clone)]
pub struct NodePos {
    node: graph::Node,
    pos: graph::Pos,
}

pub struct Layout {
    nodes: Vec<graph::Node>,
    edges: Vec<(graph::NodeId, graph::NodeId, graph::Edge)>,
    sim: Simulation,
}

impl From<graph::Node> for Node {
    fn from(_node: graph::Node) -> Self {
        Node::default()
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct NodesEdges {
    pub nodes: Vec<NodePos>,
    pub edges: Vec<(graph::NodeId, graph::NodeId, graph::Edge)>,
}

impl Layout {
    pub fn new(g: &graph::Graph) -> Result<Self> {
        let edges = g.graph.edge_references();
        let nodes: Vec<graph::Node> = g
            .graph
            .node_references()
            .into_iter()
            .map(|(_node_index, node)| node.clone())
            .collect();
        let sim = SimulationBuilder::default()
            .build(nodes.iter().map(|node| node.layout_node()))
            .add_force(
                "link",
                Link::new(edges.clone().into_iter().map(|edge| (
		    edge.source().index(),
		    edge.target().index(),
		)))
                    .strength(1.0)
                    .distance(30.0)
                    .iterations(1),
            )
	    .add_force("charge", ManyBody::new());
	let resolve = |edge: petgraph::graph::EdgeReference<graph::Edge, u32>| -> Result<_> {
	    Ok((
                g.resolve_node_id(edge.source())?,
                g.resolve_node_id(edge.target())?,
                edge.weight().clone(),
	    ))
	};
	let edges: Result<Vec<_>> = g
                .graph
                .edge_references()
                .map(resolve)
                .collect();
        Ok(Layout {
            sim,
            nodes,
            edges: edges?,
        })
    }

    pub fn step(&mut self) -> NodesEdges {
	self.sim.tick(1usize);

	let positions = self.sim.positions();

        let nodes =
            std::iter::zip(self.nodes.iter(), positions).map(|(node, pos)| NodePos {
                node: node.clone(),
                pos: graph::Pos(pos[0], pos[1]),
            });

        NodesEdges {
            nodes: nodes.collect(),
            edges: self.edges.clone(),
        }
    }

    pub fn apply(nodes_edges: &NodesEdges, graph: &mut graph::Graph) -> Result<(), Error> {
	for node_pos in &nodes_edges.nodes {
	    let node = graph.get_node_mut(&node_pos.node.id)?;
	    node.set_pos(node_pos.pos.clone());
	}
	Ok(())
    }

}

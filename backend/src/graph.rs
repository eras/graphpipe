use petgraph::Graph as PetGraph;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};

use crate::stable_ids::StableIdAllocator;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct NodeId(u32);

impl From<u32> for NodeId {
    fn from(value: u32) -> Self {
        NodeId(value)
    }
}

impl From<NodeId> for u32 {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct EdgeId(u32);

impl From<u32> for EdgeId {
    fn from(value: u32) -> Self {
        EdgeId(value)
    }
}

impl From<EdgeId> for u32 {
    fn from(id: EdgeId) -> Self {
        id.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeData {
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub id: NodeId,

    pub data: NodeData,
}

impl Node {
    pub fn layout_node(&self) -> fjadra::Node {
	fjadra::Node::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Edge {
    pub id: EdgeId,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub graph: PetGraph<Node, Edge>,
    node_id_gen: StableIdAllocator<NodeId>,
    edge_id_gen: StableIdAllocator<EdgeId>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            graph: PetGraph::new(),
            node_id_gen: StableIdAllocator::new(),
            edge_id_gen: StableIdAllocator::new(),
        }
    }

    fn new_node_index(&mut self) -> NodeId {
        return self.node_id_gen.acquire_id();
    }

    fn new_edge_index(&mut self) -> EdgeId {
        return self.edge_id_gen.acquire_id();
    }

    pub fn add_node(&mut self, node: Node) {
        let _node_index = self.graph.add_node(node);
    }

    pub fn resolve_node_index(&self, node_id: NodeId) -> NodeIndex {
        // TODO: actually resolve some stuff
        From::from(node_id.0)
    }

    pub fn resolve_node_id(&self, node_index: NodeIndex) -> NodeId {
        // TODO: actually resolve some stuff
        NodeId(node_index.index() as u32)
    }

    pub fn add_edge(&mut self, a: NodeId, b: NodeId, edge: Edge) {
        let id = self.new_edge_index();
        let edge = Edge { id };

        let _edge_index = self.graph.add_edge(
            self.resolve_node_index(a),
            self.resolve_node_index(b),
            edge,
        );
    }
}

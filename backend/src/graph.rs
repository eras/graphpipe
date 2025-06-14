use petgraph::Graph as PetGraph;
use petgraph::graph::{NodeIndex, EdgeIndex};
use serde::{Deserialize, Serialize};
use std::str::FromStr as _;
use bimap::BiMap;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Edge not found: {0}")]
    EdgeNotFound(String),

    #[error("Internal error: node index {0} not found")]
    NodeIndexNotFound(usize),

    #[error("Unsupported edge node type")]
    UnsupportedEdgeNode,

    #[error(transparent)]
    GraphvizParseError(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

use crate::stable_ids::StableIdAllocator;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct NodeId(String);

impl Eq for NodeId {}

impl From<String> for NodeId {
    fn from(value: String) -> Self {
        NodeId(value)
    }
}

impl From<NodeId> for String {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct EdgeId(String);

impl Eq for EdgeId {}

impl From<String> for EdgeId {
    fn from(value: String) -> Self {
        EdgeId(value)
    }
}

impl From<EdgeId> for String {
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
    node_id_map: BiMap<NodeId, NodeIndex>,
    edge_id_map: BiMap<EdgeId, EdgeIndex>,
    id_counter: usize,
    node_index_gen: StableIdAllocator<NodeIndex>,
    edge_index_gen: StableIdAllocator<EdgeIndex>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            graph: PetGraph::new(),
	    node_id_map: BiMap::new(),
	    edge_id_map: BiMap::new(),
	    id_counter: 0usize,
	    node_index_gen: StableIdAllocator::new(),
	    edge_index_gen: StableIdAllocator::new(),
        }
    }

    // Note! This function does not update node_id_map, you need to do it yourself
    #[allow(dead_code)]
    fn new_node_id(&mut self) -> NodeId {
	loop {
	    self.id_counter += 1;
            let id = NodeId(format!("_gpn{}", self.id_counter));
	    if !self.node_id_map.contains_left(&id) {
		//self.node_id_map.insert(id.clone(), self.node_index_gen.acquire_id());
		return id
	    }
	}
    }

    // Note! This function does not update edge_id_map, you need to do it yourself
    fn new_edge_id(&mut self) -> EdgeId {
	loop {
	    self.id_counter += 1;
            let id = EdgeId(format!("_gpe{}", self.id_counter));
	    if !self.edge_id_map.contains_left(&id) {
		//self.edge_id_map.insert(id.clone(), self.edge_index_gen.acquire_id());
		return id
	    }
	}
    }

    pub fn add_node(&mut self, node: Node) {
	// TODO: check existing node
	let node_id = node.id.clone();
        let node_index = self.graph.add_node(node);
	self.node_id_map.insert(node_id, node_index);
    }

    pub fn ensure_node(&mut self, node_id: &NodeId) {
	if let Some(_node_index) = self.node_id_map.get_by_left(node_id) {
	    // OK
	} else {
	    let node = Node { id: node_id.clone(), data: NodeData {label: node_id.0.clone()} };
            let node_index = self.graph.add_node(node);
	    self.node_id_map.insert(node_id.clone(), node_index);
	}
    }

    pub fn resolve_node_index(&self, node_id: NodeId) -> Result<NodeIndex> {
	Ok(self.node_id_map.get_by_left(&node_id).ok_or(Error::NodeNotFound(node_id.0.clone()))?.clone())
    }

    pub fn resolve_node_id(&self, node_index: NodeIndex) -> Result<NodeId> {
	Ok(self.node_id_map.get_by_right(&node_index).ok_or(Error::NodeIndexNotFound(node_index.index()))?.clone())
    }

    #[allow(dead_code)]
    pub fn resolve_edge_index(&self, edge_id: EdgeId) -> Result<EdgeIndex> {
	Ok(self.edge_id_map.get_by_left(&edge_id).ok_or(Error::EdgeNotFound(edge_id.0.clone()))?.clone())
    }

    #[allow(dead_code)]
    pub fn resolve_edge_id(&self, edge_index: EdgeIndex) -> Result<EdgeId> {
	Ok(self.edge_id_map.get_by_right(&edge_index).ok_or(Error::NodeIndexNotFound(edge_index.index()))?.clone())
    }

    pub fn add_edge(&mut self, a: NodeId, b: NodeId, _edge: Edge) -> Result<()> {
        let edge_id = self.new_edge_id();
        let edge = Edge { id: edge_id.clone() };

        let edge_index = self.graph.add_edge(
            self.resolve_node_index(a)?,
            self.resolve_node_index(b)?,
            edge,
        );
	self.edge_id_map.insert(edge_id, edge_index);
	Ok(())
    }

    pub fn parse_graphviz(&mut self, data: &str) -> Result<(), Error> {
	let ast = graphviz_parser::DotGraph::from_str(&data)?;
	if let graphviz_parser::DotGraph::Directed(graph) = ast {
	    use graphviz_parser::ast_nodes::Statement;
	    use graphviz_parser::ast_nodes::{EdgeLHS, EdgeRHS};
	    for statement in graph.statements {
		match statement {
		    Statement::Node(n) => {
			let node =
			    Node { id: NodeId(n.id.clone()),
				   data: NodeData { label: n.id.clone() } };
			self.add_node(node);
		    },
		    Statement::Edge(e) => {
			let edge = Edge { id: self.new_edge_id() };
			let lhs_id = match e.lhs {
			    EdgeLHS::Node(node) => NodeId(node.id),
			    _ => return Err(Error::UnsupportedEdgeNode)
			};
			let rhs_id = match *e.rhs {
			    EdgeRHS::Node(node) => NodeId(node.id),
			    _ => return Err(Error::UnsupportedEdgeNode)
			};
			self.ensure_node(&lhs_id);
			self.ensure_node(&rhs_id);
			self.add_edge(lhs_id, rhs_id, edge).unwrap();
		    },
		    _ => {
			// Ignore others
		    }
		}
	    }
	    //assert_eq!(node_ids, vec!["a", "b", "c"]);
	}

	Ok(())
    }
}

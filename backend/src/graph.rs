use petgraph::Graph as PetGraph;
use petgraph::graph::{NodeIndex, EdgeIndex};
use serde::{Deserialize, Serialize};
use bimap::BiMap;
use std::backtrace::Backtrace;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Node not found: {id}")]
    NodeNotFound{
	id: String,
	backtrace: Backtrace,
    },

    #[error("Edge not found: {id}")]
    EdgeNotFound{
	id: String,
	backtrace: Backtrace,
    },

    #[error("Internal error: node index {index} not found")]
    NodeIndexNotFound{
	index: usize,
	backtrace: Backtrace,
    },

    #[error("Internal error: edge index {index} not found")]
    EdgeIndexNotFound{
	index: usize,
	backtrace: Backtrace,
    },

    #[error(transparent)]
    PestError(#[from] dot_parser::ast::PestError),
}

impl Error {
    fn node_not_found(id: &str) -> Error {
	Error::NodeNotFound { id: String::from(id), backtrace: Backtrace::capture() }
    }

    fn edge_not_found(id: &str) -> Error {
	Error::EdgeNotFound { id: String::from(id), backtrace: Backtrace::capture() }
    }

    fn node_index_not_found(index: usize) -> Error {
	Error::NodeIndexNotFound { index, backtrace: Backtrace::capture() }
    }

    fn edge_index_not_found(index: usize) -> Error {
	Error::EdgeIndexNotFound { index, backtrace: Backtrace::capture() }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

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
pub struct Location(pub f64, pub f64);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub data: NodeData,
    location: Option<Location>,
}

impl Node {
    pub fn layout_node(&self) -> fjadra::Node {
	let node = fjadra::Node::default();
	let node =
	    if let Some(Location(x, y)) = &self.location {
		node.position(x.clone(), y.clone())
	    }
	    else
	    { node };
	node
    }

    pub fn set_location(&mut self, location: Location) {
	self.location = Some(location);
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
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            graph: PetGraph::new(),
	    node_id_map: BiMap::new(),
	    edge_id_map: BiMap::new(),
	    id_counter: 0usize,
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
	let node_id = node.id.clone();
	let node_index =
	    if let Some(node_index) = self.node_id_map.get_by_left(&node_id) {
		node_index.clone()
	    } else {
		self.graph.add_node(node)
	    };
	self.node_id_map.insert(node_id, node_index);
    }

    pub fn ensure_node(&mut self, node_id: &NodeId) {
	if let Some(_node_index) = self.node_id_map.get_by_left(node_id) {
	    // OK
	} else {
	    let node = Node {
		id: node_id.clone(),
		data: NodeData {label: node_id.0.clone()},
		location: None,
	    };
            let node_index = self.graph.add_node(node);
	    self.node_id_map.insert(node_id.clone(), node_index);
	}
    }

    pub fn get_node_mut(&mut self, node_id: &NodeId) -> Result<&mut Node> {
	let node_index = self.resolve_node_index(node_id)?;
	Ok(self.graph.node_weight_mut(node_index).ok_or(Error::node_not_found(&node_id.0))?)
    }

    pub fn resolve_node_index(&self, node_id: &NodeId) -> Result<NodeIndex> {
	Ok(self.node_id_map.get_by_left(node_id).ok_or(Error::node_not_found(&node_id.0))?.clone())
    }

    pub fn resolve_node_id(&self, node_index: NodeIndex) -> Result<NodeId> {
	Ok(self.node_id_map.get_by_right(&node_index).ok_or(Error::node_index_not_found(node_index.index()))?.clone())
    }

    #[allow(dead_code)]
    pub fn resolve_edge_index(&self, edge_id: EdgeId) -> Result<EdgeIndex> {
	Ok(self.edge_id_map.get_by_left(&edge_id).ok_or(Error::edge_not_found(&edge_id.0))?.clone())
    }

    #[allow(dead_code)]
    pub fn resolve_edge_id(&self, edge_index: EdgeIndex) -> Result<EdgeId> {
	Ok(self.edge_id_map.get_by_right(&edge_index).ok_or(Error::edge_index_not_found(edge_index.index()))?.clone())
    }

    pub fn add_edge(&mut self, a: NodeId, b: NodeId, _edge: Edge) -> Result<()> {
        let edge_id = self.new_edge_id();
        let edge = Edge { id: edge_id.clone() };

        let edge_index = self.graph.add_edge(
            self.resolve_node_index(&a)?,
            self.resolve_node_index(&b)?,
            edge,
        );
	self.edge_id_map.insert(edge_id, edge_index);
	Ok(())
    }

    pub fn parse_graphviz(&mut self, data: &str) -> Result<(), Error> {
	let ast = dot_parser::ast::Graph::try_from(data)?;
	let canonical = dot_parser::canonical::Graph::from(ast);

	for node in &canonical.nodes.set {
	    let gnode =
		Node {
		    id: NodeId(node.0.clone()),
		    data: NodeData { label: node.0.clone() },
		    location: None,
		};
	    self.add_node(gnode);
	}
	for edge in &canonical.edges.set {
	    let gedge = Edge { id: self.new_edge_id() };
	    let id_a = NodeId(edge.from.clone());
	    let id_b = NodeId(edge.to.clone());
	    self.ensure_node(&id_a);
	    self.ensure_node(&id_b);
	    self.add_edge(id_a, id_b, gedge).unwrap();
	}
	Ok(())
    }
}

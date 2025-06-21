use bimap::BiMap;
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Graph as PetGraph;
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::str::FromStr as _;
use std::time::SystemTime;

#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Node not found: {id}")]
    NodeNotFound { id: String, backtrace: Backtrace },

    #[error("Edge not found: {id}")]
    EdgeNotFound { id: String, backtrace: Backtrace },

    #[error("Internal error: node index {index} not found")]
    NodeIndexNotFound { index: usize, backtrace: Backtrace },

    #[error("Internal error: edge index {index} not found")]
    EdgeIndexNotFound { index: usize, backtrace: Backtrace },

    #[error("Unsupported edge node type")]
    UnsupportedEdgeNode,

    #[error(transparent)]
    GraphvizParseError(#[from] anyhow::Error),
}

impl Error {
    fn node_not_found(id: &str) -> Error {
        Error::NodeNotFound {
            id: String::from(id),
            backtrace: Backtrace::capture(),
        }
    }

    fn edge_not_found(id: &str) -> Error {
        Error::EdgeNotFound {
            id: String::from(id),
            backtrace: Backtrace::capture(),
        }
    }

    fn node_index_not_found(index: usize) -> Error {
        Error::NodeIndexNotFound {
            index,
            backtrace: Backtrace::capture(),
        }
    }

    fn edge_index_not_found(index: usize) -> Error {
        Error::EdgeIndexNotFound {
            index,
            backtrace: Backtrace::capture(),
        }
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
pub struct Pos(pub f64, pub f64);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub data: NodeData,
    pub pos: Option<Pos>,
}

impl Node {
    pub fn layout_node(&self) -> fjadra::Node {
        let node = fjadra::Node::default();
        if let Some(Pos(x, y)) = &self.pos {
            node.position(*x, *y)
        } else {
            node
        }
    }

    pub fn set_pos(&mut self, pos: Pos) {
        self.pos = Some(pos);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Edge {
    pub id: EdgeId,
}

pub type PetGraphType = PetGraph<Node, Edge>;

#[derive(Debug, Clone)]
pub struct Graph {
    pub graph: PetGraphType,
    node_id_map: BiMap<NodeId, NodeIndex>,
    edge_id_map: BiMap<EdgeId, EdgeIndex>,
    id_counter: usize,
    creation_time: SystemTime,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct GraphResponse {
    pub nodes: Vec<Node>,
    pub edges: Vec<(NodeId, NodeId, Edge)>,
    pub creation_time: f64,
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            graph: PetGraph::new(),
            node_id_map: BiMap::new(),
            edge_id_map: BiMap::new(),
            id_counter: 0usize,
            creation_time: SystemTime::now(),
        }
    }

    pub fn graph_response(&self) -> GraphResponse {
        let nodes: Vec<_> = self.graph.node_weights().cloned().collect();
        let edges: Vec<_> = self
            .graph
            .edge_references()
            .map(|edge| {
                (
                    self.resolve_node_id(edge.source())
                        .expect("Edge source missing"),
                    self.resolve_node_id(edge.target())
                        .expect("Edge target missing"),
                    edge.weight().clone(),
                )
            })
            .collect();
        let creation_time = self
            .creation_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        GraphResponse {
            nodes,
            edges,
            creation_time,
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
                return id;
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
                return id;
            }
        }
    }

    pub fn add_node(&mut self, node: Node) {
        let node_id = node.id.clone();
        let node_index = if let Some(node_index) = self.node_id_map.get_by_left(&node_id) {
            *node_index
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
                data: NodeData {
                    label: node_id.0.clone(),
                },
                pos: None,
            };
            self.add_node(node);
        }
    }

    pub fn get_node_mut(&mut self, node_id: &NodeId) -> Result<&mut Node> {
        let node_index = self.resolve_node_index(node_id)?;
        self
            .graph
            .node_weight_mut(node_index)
            .ok_or(Error::node_not_found(&node_id.0))
    }

    pub fn node_neighbors(&self, node_id: &NodeId) -> Result<Vec<&Node>> {
        let node_index = self.resolve_node_index(node_id)?;
        self
            .graph
            .neighbors_undirected(node_index)
            .map(|node_index| {
                self.graph
                    .node_weight(node_index)
                    .ok_or(Error::node_index_not_found(node_index.index()))
            })
            .collect()
    }

    pub fn resolve_node_index(&self, node_id: &NodeId) -> Result<NodeIndex> {
        Ok(*self
            .node_id_map
            .get_by_left(node_id)
            .ok_or(Error::node_not_found(&node_id.0))?
            )
    }

    pub fn resolve_node_id(&self, node_index: NodeIndex) -> Result<NodeId> {
        Ok(self
            .node_id_map
            .get_by_right(&node_index)
            .ok_or(Error::node_index_not_found(node_index.index()))?
            .clone())
    }

    #[allow(dead_code)]
    pub fn resolve_edge_index(&self, edge_id: EdgeId) -> Result<EdgeIndex> {
        Ok(*self
            .edge_id_map
            .get_by_left(&edge_id)
            .ok_or(Error::edge_not_found(&edge_id.0))?
            )
    }

    #[allow(dead_code)]
    pub fn resolve_edge_id(&self, edge_index: EdgeIndex) -> Result<EdgeId> {
        Ok(self
            .edge_id_map
            .get_by_right(&edge_index)
            .ok_or(Error::edge_index_not_found(edge_index.index()))?
            .clone())
    }

    pub fn add_edge(&mut self, a: NodeId, b: NodeId, edge_id: Option<EdgeId>) -> Result<()> {
        let edge_id = edge_id.unwrap_or_else(|| self.new_edge_id());
        let edge = Edge {
            id: edge_id.clone(),
        };

        let edge_index = self.graph.add_edge(
            self.resolve_node_index(&a)?,
            self.resolve_node_index(&b)?,
            edge,
        );
        self.edge_id_map.insert(edge_id, edge_index);
        Ok(())
    }

    pub fn parse_graphviz(&mut self, data: &str) -> Result<(), Error> {
        let ast = graphviz_parser::DotGraph::from_str(data)?;
        if let graphviz_parser::DotGraph::Directed(graph) = ast {
            use graphviz_parser::ast_nodes::Statement;
            use graphviz_parser::ast_nodes::{EdgeLHS, EdgeRHS};
            for statement in graph.statements {
                match statement {
                    Statement::Node(n) => {
                        let attrs = attr_map(&n.attribute_list);
                        let node = Node {
                            id: NodeId(n.id.clone()),
                            data: NodeData {
                                label: attrs.get("label").unwrap_or(&&n.id).to_string(),
                            },
                            pos: None,
                        };
                        self.add_node(node);
                    }
                    Statement::Edge(e) => {
                        let edge_id = self.new_edge_id();
                        let lhs_id = match e.lhs {
                            EdgeLHS::Node(node) => NodeId(node.id),
                            _ => return Err(Error::UnsupportedEdgeNode),
                        };
                        let rhs_id = match *e.rhs {
                            EdgeRHS::Node(node) => NodeId(node.id),
                            _ => return Err(Error::UnsupportedEdgeNode),
                        };
                        self.ensure_node(&lhs_id);
                        self.ensure_node(&rhs_id);
                        self.add_edge(lhs_id, rhs_id, Some (edge_id)).unwrap();
                    }
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

fn attr_map(
    attr_list: &Option<graphviz_parser::ast_nodes::AttributeList>,
) -> HashMap<&str, &String> {
    let mut attrs = HashMap::new();
    if let Some(attribute_list) = attr_list {
        for attr_group in attribute_list {
            for assignment in attr_group {
                attrs.insert(assignment.lhs.as_str(), &assignment.rhs);
            }
        }
    }
    attrs
}

use std::time::SystemTime;
use actix_web::{
    App, HttpServer, Responder,
    middleware::Logger,
    web::{self, Data},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use std::backtrace::Backtrace;

use crate::graph::{Edge, Graph, Node, NodeId};
use crate::layout::{Layout, NodePos};

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Graph error: {source}")]
    GraphError {
	#[from] source: crate::graph::Error,
	backtrace: Backtrace,
    },

    #[error("Layout error: {source}")]
    LayoutError{
	#[from] source: crate::layout::Error,
	backtrace: Backtrace,
    },
}

impl Error {
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            Error::GraphError { backtrace, .. } => Some(backtrace),
            Error::LayoutError { backtrace, .. } => Some(backtrace),
        }
    }
}

impl actix_web::ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
	actix_web::HttpResponse::build(self.status_code())
            .insert_header(actix_web::http::header::ContentType::html())
            .body(format!("{}. Backtrace: {:?}", &self, self.backtrace()))
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
	actix_web::http::StatusCode::from_u16(400u16).unwrap()
    }
}

struct GraphData {
    graph: Graph,
    creation_time: SystemTime,
    layout: Option<Layout>,
}

impl GraphData {
    fn reset_layout(&mut self) {
	self.layout = None;
    }
    fn update_layout(&mut self) -> Result<&mut Layout, Error> {
	if self.layout.is_none() {
	    self.layout = Some(Layout::new(&self.graph)?);
	}
	Ok(self.layout.as_mut().unwrap())
    }
}

#[derive(Serialize, Debug, Clone)]
struct NodesEdgesInfo {
    nodes: Vec<NodePos>,
    edges: Vec<(NodeId, NodeId, Edge)>,
    creation_time: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EdgeRequest {
    a: NodeId,
    b: NodeId,
    edge: Edge,
}

fn no_nodes() -> Vec<Node> { Vec::new() }

fn no_edge_requests() -> Vec<EdgeRequest> { Vec::new() }

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AddRequest {
    #[serde(default = "no_nodes")]
    nodes: Vec<Node>,
    #[serde(default = "no_edge_requests")]
    edges: Vec<EdgeRequest>,
}

#[actix_web::get("/graph")]
async fn list(data: Data<Mutex<GraphData>>) -> impl Responder {
    let data = data.lock().await;

    // for (node_idx, node_data) in data.graph.graph.node_references() {
    //     println!("  Node Index: {:?}, Data: '{:?}'", node_idx, node_data);
    // }

    web::Json(data.graph.graph.clone())
}

#[actix_web::post("/graph")]
async fn add(data: Data<Mutex<GraphData>>, request: web::Json<AddRequest>) -> actix_web::Result<web::Json<Option<String>>, Error> {
    let mut data = data.lock().await;
    data.reset_layout();
    let request = request.into_inner();
    for node in request.nodes {
        data.graph.add_node(node)
    }
    for edge in request.edges {
        data.graph.add_edge(edge.a, edge.b, edge.edge)?
    }
    Ok(web::Json(None::<String>))
}

#[actix_web::post("/graph/graphviz")]
async fn post_graphviz(data: Data<Mutex<GraphData>>, body: String) -> actix_web::Result<String> {
    let mut data = data.lock().await;
    data.reset_layout();
    match data.graph.parse_graphviz(&body) {
	Ok(()) => {
	    Ok(format!(""))
	},
	Err(error) => {
	    Err(actix_web::error::ErrorBadRequest(format!("Parse error: {:?}", error)))
	}
    }
}

#[actix_web::get("/graph/layout")]
async fn layout(data: Data<Mutex<GraphData>>) -> actix_web::Result<web::Json<NodesEdgesInfo>, Error> {
    let mut data = data.lock().await;

    let layout = data.update_layout()?;
    let nodes_edges = layout.step();
    Layout::apply(&nodes_edges, &mut data.graph)?;

    let response = NodesEdgesInfo {
	nodes: nodes_edges.nodes,
	edges: nodes_edges.edges,
	creation_time: data.creation_time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64(),
    };

    Ok(web::Json(response))
}

pub async fn main() -> std::io::Result<()> {
    let graph = Graph::new();
    let creation_time = SystemTime::now();
    let data = Data::new(Mutex::new(GraphData { graph, creation_time, layout: None }));

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(Data::clone(&data))
            .service(list)
            .service(add)
            .service(post_graphviz)
            .service(layout)
	    .service(actix_files::Files::new("/", "./backend/assets")
                     .index_file("index.html") // Specifies the default file for directory requests
                     // .show_files_listing() // Optional: Enable to show directory listings if no index file
            )

    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await

    //Ok(())
}

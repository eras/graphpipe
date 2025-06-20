use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer,
};
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::bg_layout::BgLayout;
use crate::graph::{Edge, Graph, GraphResponse, Node, NodeId};
use crate::graph_data::{GraphData, GraphDataType};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Graph data error: {source}")]
    GraphDataError {
        #[from]
        source: crate::graph_data::Error,
        backtrace: Backtrace,
    },

    #[error("Graph error: {source}")]
    GraphError {
        #[from]
        source: crate::graph::Error,
        backtrace: Backtrace,
    },

    #[error("Layout error: {source}")]
    LayoutError {
        #[from]
        source: crate::layout::Error,
        backtrace: Backtrace,
    },
}

impl Error {
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            Error::GraphDataError { backtrace, .. } => Some(backtrace),
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EdgeRequest {
    a: NodeId,
    b: NodeId,
    edge: Edge,
}

fn no_nodes() -> Vec<Node> {
    Vec::new()
}

fn no_edge_requests() -> Vec<EdgeRequest> {
    Vec::new()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AddRequest {
    #[serde(default = "no_nodes")]
    nodes: Vec<Node>,
    #[serde(default = "no_edge_requests")]
    edges: Vec<EdgeRequest>,
}

#[actix_web::get("/graph")]
async fn list(data: Data<GraphDataType>) -> actix_web::Result<web::Json<GraphResponse>, Error> {
    let data = data.lock().await;
    let nodes_edges = data.graph.graph_response();
    Ok(web::Json(nodes_edges))
}

#[actix_web::post("/graph")]
async fn add(
    data: Data<GraphDataType>,
    request: web::Json<AddRequest>,
) -> actix_web::Result<web::Json<Option<String>>, Error> {
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
async fn post_graphviz(data: Data<GraphDataType>, body: String) -> actix_web::Result<String> {
    let mut data = data.lock().await;
    data.reset_layout();
    match data.graph.parse_graphviz(&body) {
        Ok(()) => Ok(format!("")),
        Err(error) => Err(actix_web::error::ErrorBadRequest(format!(
            "Parse error: {:?}",
            error
        ))),
    }
}

pub async fn main() -> std::io::Result<()> {
    let graph = Graph::new();
    let graph_data = Arc::new(Mutex::new(GraphData {
        graph,
        layout: None,
    }));
    let data = Data::new(graph_data.clone());

    let bg_layout = BgLayout::new(graph_data.clone());
    bg_layout.start();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(Data::clone(&data))
            .service(list)
            .service(add)
            .service(post_graphviz)
            .service(
                actix_files::Files::new("/", "./backend/assets").index_file("index.html"), // Specifies the default file for directory requests
                                                                                           // .show_files_listing() // Optional: Enable to show directory listings if no index file
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await

    //Ok(())
}

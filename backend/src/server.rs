use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::net::SocketAddr;
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

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Address and port to listen on, e.g., "127.0.0.1:8080" or "8080"
    #[clap(long)]
    listen: Option<String>,
}

// Function to configure and run the Actix-web server
async fn run_server(
    listen_addr: SocketAddr,
    data: web::Data<GraphDataType>,
) -> std::io::Result<()> {
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(data.clone())
            .service(list)
            .service(add)
            .service(post_graphviz)
            .service(actix_files::Files::new("/", "./backend/assets").index_file("index.html"))
    })
    .bind(listen_addr)?;
    for addr in server.addrs() {
        println!("Started server on http://{}", addr);
    }
    server.run().await
}

// Function to handle the listening address logic
fn get_listen_address(
    listen_arg: Option<String>,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let default_port = 0;
    let default_host = "127.0.0.1";

    match listen_arg {
        Some(addr_str) => {
            if let Ok(socket_addr) = addr_str.parse::<SocketAddr>() {
                // Full address provided (e.g., "127.0.0.1:8080")
                Ok(socket_addr)
            } else if let Ok(port) = addr_str.parse::<u16>() {
                // Only port provided (e.g., "8080")
                let addr = format!("{}:{}", default_host, port);
                Ok(addr.parse::<SocketAddr>()?)
            } else {
                Err(format!("Invalid listen address format: {}", addr_str).into())
            }
        }
        None => {
            // No --listen argument, use default
            let addr = format!("{}:{}", default_host, default_port);
            Ok(addr.parse::<SocketAddr>()?)
        }
    }
}

pub async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let graph = Graph::new();
    let graph_data = Arc::new(Mutex::new(GraphData {
        graph,
        layout: None,
    }));
    let data = web::Data::new(graph_data.clone()); // Wrap in web::Data

    let bg_layout = BgLayout::new(graph_data.clone());
    bg_layout.start();

    let listen_addr = get_listen_address(args.listen)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

    run_server(listen_addr, data).await
}

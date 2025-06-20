use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::net::{SocketAddr, ToSocketAddrs};
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

    #[error("Layout error: {source}")]
    LocalIpAddressError {
        #[from]
        source: local_ip_address::Error,
    },

    #[error("IO error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },

    #[error("Address parse error: {source}")]
    AddrParseError {
        #[from]
        source: std::net::AddrParseError,
    },

    #[error("Failed to resolve address: {message}")]
    AddressError { message: String },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            Error::GraphDataError { backtrace, .. } => Some(backtrace),
            Error::GraphError { backtrace, .. } => Some(backtrace),
            Error::LayoutError { backtrace, .. } => Some(backtrace),
            Error::LocalIpAddressError { .. } => None,
            Error::IOError { .. } => None,
            Error::AddrParseError { .. } => None,
            Error::AddressError { .. } => None,
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
        data.graph.ensure_node(&edge.a);
        data.graph.ensure_node(&edge.b);
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
    /// Address and port to listen on, e.g., "127.0.0.1:8080", "hostname:8080", "8080", or "hostname:0", "0" for dynamic port
    #[clap(long)]
    listen: Option<String>,
}

// Function to configure and run the Actix-web server
async fn run_server(listen_addr: SocketAddr, data: web::Data<GraphDataType>) -> Result<()> {
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
        if addr.ip().is_unspecified() {
            // Now, get all local network interfaces and print their IP addresses
            let network_interfaces = local_ip_address::list_afinet_netifas()?;

            for (name, ip) in network_interfaces.iter() {
                if !ip.is_unspecified() {
                    println!("Started server on http://{}:{} ({})", ip, addr.port(), name);
                }
            }
        } else {
            println!("Started server on http://{}", addr);
        }
    }
    Ok(server.run().await?)
}

// Function to handle the listening address logic
fn get_listen_address(listen_arg: Option<String>) -> Result<SocketAddr> {
    let default_port = 0;
    let default_host = "127.0.0.1";

    match listen_arg {
        Some(addr_str) => {
            // Attempt to parse as a direct SocketAddr first (e.g., "127.0.0.1:8080", "[::1]:8080")
            if let Ok(socket_addr) = addr_str.parse::<SocketAddr>() {
                Ok(socket_addr)
            }
            // If not a direct SocketAddr, try parsing as just a port (e.g., "8080")
            else if let Ok(port) = addr_str.parse::<u16>() {
                // If only port is provided, use default_host with that port
                let bind_tuple = (default_host, port);
                let mut addrs = bind_tuple
                    .to_socket_addrs()
                    .map_err(|e| Error::AddressError {
                        message: format!(
                            "Failed to resolve default host '{}': {}",
                            default_host, e
                        ),
                    })?;
                addrs.next().ok_or_else(|| Error::AddressError {
                    message: format!(
                        "No addresses found for default host '{}:{}'",
                        default_host, port
                    ),
                })
            }
            // If neither, try parsing as hostname:port (e.g., "localhost:8080", "example.com:0")
            else {
                // Split the string into host and port parts
                let parts: Vec<&str> = addr_str.split(':').collect();
                if parts.len() == 2 {
                    let host = parts[0];
                    let port_str = parts[1];

                    let port: u16 = port_str.parse().map_err(|e| Error::AddressError {
                        message: format!("Invalid port in '{}': {}", addr_str, e),
                    })?;

                    let bind_tuple = (host, port);
                    let mut addrs =
                        bind_tuple
                            .to_socket_addrs()
                            .map_err(|e| Error::AddressError {
                                message: format!(
                                    "Failed to resolve host '{}:{}': {}",
                                    host, port, e
                                ),
                            })?;
                    addrs.next().ok_or_else(|| Error::AddressError {
                        message: format!("No addresses found for '{}:{}'", host, port),
                    })
                } else {
                    Err(Error::AddressError {
                        message: format!(
                            "Invalid listen address format. Expected 'host:port' or 'port': {}",
                            addr_str
                        ),
                    })
                }
            }
        }
        None => {
            // No --listen argument, use default host and default port (0 for dynamic)
            let bind_tuple = (default_host, default_port);
            let mut addrs = bind_tuple
                .to_socket_addrs()
                .map_err(|e| Error::AddressError {
                    message: format!("Failed to resolve default host '{}': {}", default_host, e),
                })?;
            addrs.next().ok_or_else(|| Error::AddressError {
                message: format!(
                    "No addresses found for default host '{}:{}'",
                    default_host, default_port
                ),
            })
        }
    }
}

pub async fn main() -> Result<()> {
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

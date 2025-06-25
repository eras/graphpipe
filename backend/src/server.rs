use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer, Responder,
};
use serde::{Deserialize, Serialize};
use std::{backtrace::Backtrace, time::Duration};
use std::{convert::Infallible, net::SocketAddr};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt; // For stream combinators like .next()

use std::sync::Arc;

use crate::{
    assets,
    graph::{EdgeId, GraphResponse, Node, NodeId},
};
use crate::{bg_layout, graph_data::GraphDataType};

#[allow(clippy::enum_variant_names)]
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

    #[error("IO error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            Error::GraphDataError { backtrace, .. } => Some(backtrace),
            Error::GraphError { backtrace, .. } => Some(backtrace),
            Error::LayoutError { backtrace, .. } => Some(backtrace),
            Error::IOError { .. } => None,
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
    id: Option<EdgeId>,
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
        data.graph.add_edge(edge.a, edge.b, edge.id)?
    }
    Ok(web::Json(None::<String>))
}

#[actix_web::post("/graphviz")]
async fn post_graphviz(data: Data<GraphDataType>, body: String) -> actix_web::Result<String> {
    let mut data = data.lock().await;
    data.reset_layout();
    match data.graph.parse_graphviz(&body) {
        Ok(()) => Ok(String::new()),
        Err(error) => Err(actix_web::error::ErrorBadRequest(format!(
            "Parse error: {error:?}",
        ))),
    }
}

#[actix_web::get("/stream")]
async fn from_channel(bg_control: web::Data<bg_layout::BgControl>) -> impl Responder {
    let updates = BroadcastStream::new(bg_control.updates());

    let events = updates.map(|update| {
        let update = update.expect("woot, there should have been an update..");
        let json_data = serde_json::to_string(&update).expect("Failed to encode Update to JSON");
        Ok::<_, Infallible>(actix_web_lab::sse::Event::Data(
            actix_web_lab::sse::Data::new(json_data),
        ))
    });

    actix_web_lab::sse::Sse::from_stream(events).with_keep_alive(Duration::from_secs(5))
}

// Function to configure and run the Actix-web server
pub async fn run_server(
    listen_addr: SocketAddr,
    data: GraphDataType,
    bg_control: bg_layout::BgControl,
    addresses: tokio::sync::oneshot::Sender<Vec<std::net::SocketAddr>>,
) -> Result<actix_web::dev::Server, Error> {
    let server = Arc::new(
        HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .app_data(web::Data::new(data.clone()))
                .app_data(web::Data::new(bg_control.clone()))
                .service(list)
                .service(add)
                .service(post_graphviz)
                .service(from_channel)
                .service(assets::assets("", "index.html"))
        })
        .bind(listen_addr)?,
    );
    let _ignore = addresses.send(server.addrs());
    Ok(Arc::into_inner(server).unwrap().run())
}

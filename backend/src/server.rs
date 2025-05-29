use actix_web::{
    App, HttpServer, Responder,
    middleware::Logger,
    web::{self, Data},
};
use petgraph::visit::IntoNodeReferences;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::graph::{Edge, Graph, Node, NodeId};
use crate::layout::Layout;

struct GraphData {
    graph: Graph,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EdgeRequest {
    a: NodeId,
    b: NodeId,
    edge: Edge,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AddRequest {
    nodes: Vec<Node>,
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
async fn add(data: Data<Mutex<GraphData>>, request: web::Json<AddRequest>) -> impl Responder {
    let mut data = data.lock().await;
    let request = request.into_inner();
    for node in request.nodes {
        data.graph.add_node(node)
    }
    for edge in request.edges {
        data.graph.add_edge(edge.a, edge.b, edge.edge)
    }
    web::Json(None::<String>)
}

#[actix_web::get("/graph/sim")]
async fn sim(data: Data<Mutex<GraphData>>) -> impl Responder {
    let data = data.lock().await;

    let mut layout = Layout::new(&data.graph);
    let nodes_edges = layout.step();
    
    web::Json(nodes_edges)
}

pub async fn main() -> std::io::Result<()> {
    let graph = Graph::new();
    let data = Data::new(Mutex::new(GraphData { graph }));

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(Data::clone(&data))
            .service(list)
            .service(add)
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

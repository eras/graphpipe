#![feature(error_generic_member_access)]

mod bg_layout;
mod graph;
mod graph_data;
mod layout;
mod server;

use actix_web::web;
use clap::Parser;
use env_logger::Env;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::bg_layout::BgLayout;
use crate::graph::Graph;
use crate::graph_data::GraphData;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to resolve address: {message}")]
    AddressError { message: String },

    #[error("IO error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },

    #[error("Server error: {source}")]
    ServerError {
        #[from]
        source: server::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Address and port to listen on, e.g., "127.0.0.1:8080", "hostname:8080", "8080", or "hostname:0", "0" for dynamic port
    #[clap(long)]
    listen: Option<String>,
}

// Function to handle the listening address logic
#[allow(clippy::result_large_err)]
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
                        message: format!("Failed to resolve default host '{default_host}': {e}"),
                    })?;
                addrs.next().ok_or_else(|| Error::AddressError {
                    message: format!("No addresses found for default host '{default_host}:{port}'"),
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
                        message: format!("Invalid port in '{addr_str}': {e}"),
                    })?;

                    let bind_tuple = (host, port);
                    let mut addrs =
                        bind_tuple
                            .to_socket_addrs()
                            .map_err(|e| Error::AddressError {
                                message: format!("Failed to resolve host '{host}:{port}': {e}"),
                            })?;
                    addrs.next().ok_or_else(|| Error::AddressError {
                        message: format!("No addresses found for '{host}:{port}'"),
                    })
                } else {
                    Err(Error::AddressError {
                        message: format!(
                            "Invalid listen address format. Expected 'host:port' or 'port': {addr_str}",
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
                    message: format!("Failed to resolve default host '{default_host}': {e}"),
                })?;
            addrs.next().ok_or_else(|| Error::AddressError {
                message: format!(
                    "No addresses found for default host '{default_host}:{default_port}'",
                ),
            })
        }
    }
}

#[allow(clippy::result_large_err)]
#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("error"));

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

    Ok(server::run_server(listen_addr, data).await?)
}

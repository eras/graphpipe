#![feature(error_generic_member_access)]

mod assets;
mod bg_layout;
mod graph;
mod graph_data;
mod layout;
mod server;

use clap::Parser;
use env_logger::Env;
use std::io::{Read, Write};
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

    #[error("Layout error: {source}")]
    LocalIpAddressError {
        #[from]
        source: local_ip_address::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Address and port to listen on, e.g., "127.0.0.1:8080", "hostname:8080", "8080", or "hostname:0", "0" for dynamic port
    #[clap(long)]
    listen: Option<String>,

    #[arg(long, default_value_t = false)]
    sh: bool,
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
async fn tokio_main(
    args: Args,
    verbose: bool,
    mut for_sh_pipe: Option<std::io::PipeWriter>,
) -> Result<()> {
    let graph = Graph::new();
    let graph_data = Arc::new(Mutex::new(GraphData {
        graph,
        layout: None,
    }));
    let data = graph_data.clone();

    let bg_layout = BgLayout::new(graph_data.clone());
    let bg_control = bg_layout.start();

    let listen_addr = get_listen_address(args.listen)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

    let (addresses_tx, addresses_rx) = tokio::sync::oneshot::channel();

    let join = tokio::spawn(async move {
        match server::run_server(listen_addr, data, bg_control, addresses_tx).await {
            Ok(x) => x.await.map_err(|err| Error::from(err)),
            Err(err) => Err(Error::from(err)),
        }
    });

    let mut sent_for_sh = false;
    let mut send_for_sh = |address: &str| {
        if let Some(pipe_writer) = &mut for_sh_pipe {
            if !sent_for_sh {
                sent_for_sh = true;
                pipe_writer
                    .write_all(
                        &format!(
                            "echo Graphpipe is serving at {address}; export GRAPHPIPE={address}\n"
                        )
                        .into_bytes(),
                    )
                    .unwrap(); // We want to crash is this write fails
            }
        }
    };

    for addr in addresses_rx.await.expect("oneshot receive should succeed") {
        if addr.ip().is_unspecified() {
            // Now, get all local network interfaces and print their IP addresses
            let network_interfaces = local_ip_address::list_afinet_netifas()?;

            for (name, ip) in network_interfaces.iter() {
                if !ip.is_unspecified() {
                    if verbose {
                        eprintln!("Started server on http://{}:{} ({})", ip, addr.port(), name);
                    }
                    send_for_sh(&format!("http://{}:{}", ip, addr.port()));
                }
            }
        } else {
            if verbose {
                eprintln!("Started server on http://{addr}");
            }
            send_for_sh(&format!("http://{}", addr));
        }
    }

    drop(for_sh_pipe);

    // We want to crash if this join fails, or if there was an error in the start
    // TODO: ..but we could maybe do it better somehow? E.g. try join before drop, to extract fast errors?
    join.await.unwrap().unwrap();

    Ok(())
}

#[allow(clippy::result_large_err)]
fn main() -> Result<()> {
    let args = Args::parse();

    if !args.sh {
        env_logger::init_from_env(Env::default().default_filter_or("error"));

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(tokio_main(args, true, None))
    } else {
        let (mut for_sh_reader, for_sh_writer) = std::io::pipe()?;
        match fork::daemon(true, true) {
            Ok(fork::Fork::Child) => {
                // SAFETY: let's hope we don't write to stdout or stderr
                unsafe { libc::close(1) };
                unsafe { libc::close(2) };
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()?
                    .block_on(tokio_main(args, false, Some(for_sh_writer)))
            }
            Ok(fork::Fork::Parent(pid)) => {
                drop(for_sh_writer);
                let mut for_sh = String::new();
                for_sh_reader.read_to_string(&mut for_sh)?;
                print!("{for_sh}");
                println!("export GRAPHPIPE_PID={pid}; echo Graphpipe process id is {pid}");
                Ok(())
            }
            Err(_) => {
                eprintln!("Failed to fork");
                Ok(()) // TODO
            }
        }
    }
}

use std::backtrace::Backtrace;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::graph::GraphResponse;
use crate::graph_data::GraphDataType;
use crate::layout::Layout;

#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
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

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct BgLayout {
    graph_data: GraphDataType,
    exit_requested: Arc<AtomicBool>,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct Update {
    graph: GraphResponse,
}

// TODO: This is for future use.. ?
#[derive(Clone)]
#[allow(dead_code)]
pub struct BgControl {
    graph_data: GraphDataType,
    exit_requested: Arc<AtomicBool>,
    updates_tx: broadcast::WeakSender<Update>,
}

impl BgControl {
    // TODO: This is for future use.. ?
    #[allow(dead_code)]
    pub fn exit(self) {
        self.exit_requested.store(true, Relaxed);
    }

    pub fn updates(&self) -> broadcast::Receiver<Update> {
        match self.updates_tx.upgrade() {
            Some(updates_tx) => updates_tx.subscribe(),
            None => todo!(),
        }
    }
}

impl BgLayout {
    pub fn new(graph_data: GraphDataType) -> BgLayout {
        let exit_requested = Arc::new(AtomicBool::new(false));
        BgLayout {
            graph_data,
            exit_requested,
        }
    }

    pub fn start(self: BgLayout) -> BgControl {
        let exit_requested = self.exit_requested.clone();
        let graph_data = self.graph_data.clone();
        let (updates_tx, _updates_rx) = broadcast::channel(10);
        let _join = tokio::spawn(self.run(updates_tx.clone()));
        BgControl {
            graph_data,
            exit_requested,
            updates_tx: updates_tx.downgrade(),
        }
    }

    async fn do_layout(self: &mut BgLayout) -> Result<(), Error> {
        let mut data = self.graph_data.lock().await;

        let layout = data.update_layout()?;
        let nodes_edges = layout.step();
        Layout::apply(&nodes_edges, &mut data.graph)?;
        Ok(())
    }

    async fn send_update(
        self: &BgLayout,
        updates_tx: &broadcast::Sender<Update>,
    ) -> Result<(), tokio::sync::broadcast::error::SendError<Update>> {
        let data = self.graph_data.lock().await;
        let update = Update {
            graph: data.graph.graph_response(),
        };
        let _subscriber_count = updates_tx.send(update)?;
        Ok(())
    }

    async fn run(mut self: BgLayout, updates_tx: broadcast::Sender<Update>) {
        while !self.exit_requested.load(Relaxed) {
            let _ = self.do_layout().await;
            tokio::time::sleep(Duration::from_millis(100)).await;

            // SendError can be ignored: it is a common case that there are no recipients
            let _ = self.send_update(&updates_tx).await;
        }
    }
}

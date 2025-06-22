use std::backtrace::Backtrace;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::Duration;

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

pub struct BgLayout {
    graph_data: GraphDataType,
    exit_requested: Arc<AtomicBool>,
}

// TODO: This is for future use.. ?
#[allow(dead_code)]
pub struct Control {
    graph_data: GraphDataType,
    exit_requested: Arc<AtomicBool>,
}

// TODO: This is for future use.. ?
#[allow(dead_code)]
impl Control {
    pub fn exit(self) {
        self.exit_requested.store(true, Relaxed);
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

    pub fn start(self: BgLayout) -> Control {
        let exit_requested = self.exit_requested.clone();
        let graph_data = self.graph_data.clone();
        let _join = tokio::spawn(self.run());
        Control {
            graph_data,
            exit_requested,
        }
    }

    async fn do_layout(self: &mut BgLayout) -> Result<(), Error> {
        let mut data = self.graph_data.lock().await;

        let layout = data.update_layout()?;
        let nodes_edges = layout.step();
        Layout::apply(&nodes_edges, &mut data.graph)?;
        Ok(())
    }

    async fn run(mut self: BgLayout) {
        while !self.exit_requested.load(Relaxed) {
            let _ = self.do_layout().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

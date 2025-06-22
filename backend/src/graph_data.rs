use std::backtrace::Backtrace;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::graph::Graph;
use crate::layout::Layout;

#[derive(thiserror::Error, Debug)]
pub enum Error {
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

pub struct GraphData {
    pub graph: Graph,
    pub layout: Option<Layout>,
}

pub type GraphDataType = Arc<Mutex<GraphData>>;

impl GraphData {
    pub fn reset_layout(&mut self) {
        self.layout = None;
    }

    pub fn is_empty(&self) -> bool {
        self.graph.graph.node_count() == 0
    }

    #[allow(clippy::result_large_err)]
    pub fn update_layout(&mut self) -> Result<&mut Layout, Error> {
        if self.layout.is_none() {
            self.layout = Some(Layout::new(&self.graph)?);
        }
        Ok(self.layout.as_mut().unwrap())
    }
}

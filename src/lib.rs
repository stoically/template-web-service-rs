#![doc = include_str!("../README.md")]

pub mod cli;
pub mod config;
mod error;
mod http;
mod state;

use std::sync::Arc;

use error_stack::{IntoReport, Result, ResultExt};
use state::{InnerState, State};
use timed_locks::RwLock;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub use crate::{config::Config, error::Error, http::app};

/// Web service.
pub struct WebService {
    state: State,
}

impl WebService {
    /// Create new [`WebService`] with default configuration.
    pub fn new() -> Self {
        let config = Config::new();
        Self::new_with_config(config)
    }

    /// Create new [`WebService`] with the given configuration.
    pub fn new_with_config(config: Config) -> Self {
        Self::tracing_subscriber(&config);
        Self {
            state: State {
                inner: Arc::new(RwLock::new(InnerState { config })),
            },
        }
    }

    /// Initialize the tracing subscriber.
    #[tracing::instrument(skip_all)]
    fn tracing_subscriber(config: &Config) {
        tracing_subscriber::fmt()
            .with_file(true)
            .with_line_number(true)
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(config.log.level_filter.into())
                    .from_env_lossy(),
            )
            .init();
    }

    /// Spawn the web service.
    ///
    /// Blocking call that tries to listen on the configured http socket
    /// address.
    #[tracing::instrument(skip(self))]
    pub async fn spawn(&self) -> Result<(), Error> {
        let addr = self.state.read().await.config.http.addr;
        info!("Listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app(self.state.clone()).into_make_service())
            .await
            .into_report()
            .change_context(Error::Hyper)?;

        Ok(())
    }
}

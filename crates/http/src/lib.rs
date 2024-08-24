use axum::{
    response::Html,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

pub mod error;

use error::Result;

#[derive(Clone)]
pub struct Server {
    addr: SocketAddr,
}

#[derive(Clone)]
pub struct ServerState {}

/// Get the default router for the API routes.
fn api_routes<S: std::clone::Clone + Send + Sync + 'static>() -> Router<S>
where
    ServerState: axum::extract::FromRef<S>,
{
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
}

impl Server {
    #[must_use]
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Run the server.
    /// This will start the api server and serve it from /api and serve static files when no route is matched.
    ///
    /// # Errors
    /// - failure to bind a `TcpListener`
    pub async fn run(self) -> Result<()> {
        info!("Starting server on {}", self.addr);

        let cors = tower_http::cors::CorsLayer::permissive();

        let state = ServerState {
        };

        let app = Router::new()
            .nest("/api", api_routes())
            .with_state(state)
            .layer(cors);

        tracing::debug!("listening on {}", &self.addr);
        let listener = TcpListener::bind(&self.addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

pub struct Builder {
    addr: Option<SocketAddr>,
}

impl Builder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            addr: None,
        }
    }

    #[must_use]
    pub fn addr(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    /// # Errors
    /// - not all require options set on the builder
    pub fn build(self) -> Result<Server> {
        let addr = self.addr.ok_or(error::Error::ServerBuilder)?;

        Ok(Server {
            addr,
        })
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            addr: Some(SocketAddr::from(([0, 0, 0, 0], 3000))),
        }
    }
}

async fn root() -> Html<String> {
    Html("<h1>Hello, world!</h1>".to_string())
}

async fn health() -> &'static str {
    "OK"
}

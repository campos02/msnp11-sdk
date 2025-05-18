use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;
use hyper::Request;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server;
use log::error;
use tower_service::Service;

pub struct MockHttp;

impl MockHttp {
    pub async fn mock_passport() {
        let app = Router::new()
            .route("/rdr/pprdr.asp", get(Self::nexus))
            .route("/login.srf", get(Self::login_srf));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
            .await
            .expect("Could not bind HTTP server");

        loop {
            let (socket, _remote_addr) = match listener.accept().await {
                Ok(l) => l,
                Err(error) => {
                    error!(": {error}");
                    continue;
                }
            };

            let tower_service = app.clone();

            tokio::spawn(async move {
                let socket = TokioIo::new(socket);
                let hyper_service =
                    hyper::service::service_fn(move |request: Request<Incoming>| {
                        tower_service.clone().call(request)
                    });

                let mut builder = server::conn::auto::Builder::new(TokioExecutor::new());
                builder.http1().title_case_headers(true);

                if let Err(err) = builder
                    .serve_connection_with_upgrades(socket, hyper_service)
                    .await
                {
                    error!("Failed to serve connection: {err:#}");
                }
            });
        }
    }

    async fn nexus() -> impl IntoResponse {
        [("PassportURLs", "DALogin=http://localhost:3000/login.srf")]
    }

    async fn login_srf() -> impl IntoResponse {
        [(
            "Authentication-Info",
            "Passport1.4 da-status=success,from-PP='aaa123aaa123'",
        )]
    }
}

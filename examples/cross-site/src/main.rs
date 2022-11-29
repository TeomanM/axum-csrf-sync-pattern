use std::net::SocketAddr;

use axum::{
    http::{header, Method, StatusCode},
    response::IntoResponse,
    routing::{get, Router},
    Server,
};
use axum_csrf_sync_pattern::CsrfSynchronizerTokenLayer;
use axum_sessions::{async_session::MemoryStore, SessionLayer};
use color_eyre::eyre::{self, eyre, WrapErr};
use rand::RngCore;
use tower_http::cors::{AllowOrigin, CorsLayer};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    // Use the "Send POST request without CSRF token" button in your browser,
    // then check your console to find "WARN axum_csrf_sync_pattern: X-CSRF-TOKEN header missing!".
    // The middleware uses tracing to log all error cases, including CSRF rejections.
    tracing_subscriber::fmt::try_init()
        .map_err(|e| eyre!(e))
        .wrap_err("Failed to initialize tracing-subscriber.")?;

    let frontend = async {
        let app = Router::new().route("/", get(index));

        // Visit "http://127.0.0.1:3000/" in your browser.
        serve(app, 3000).await?;

        Ok::<(), eyre::Report>(())
    };

    let backend = async {
        let mut secret = [0; 64];
        rand::thread_rng()
            .try_fill_bytes(&mut secret)
            .wrap_err("Failed to generate session seed.")?;

        let app = Router::new()
            .route("/", get(get_token).post(post_handler))
            .layer(CsrfSynchronizerTokenLayer::new())
            .layer(SessionLayer::new(MemoryStore::new(), &secret))
            .layer(
                CorsLayer::new()
                    .allow_origin(AllowOrigin::list([
                        // Allow CORS requests from our frontend.
                        "http://127.0.0.1:3000"
                            .parse()
                            .wrap_err("Failed to parse socket address.")?,
                    ]))
                    // Allow GET and POST methods. Adjust to your needs.
                    .allow_methods([Method::GET, Method::POST])
                    .allow_headers([
                        // Allow incoming CORS requests to use the Content-Type header,
                        header::CONTENT_TYPE,
                        // as well as the `CsrfSynchronizerTokenLayer` default request header.
                        "X-CSRF-TOKEN"
                            .parse()
                            .wrap_err("Failed to parse token header.")?,
                    ])
                    // Allow CORS requests with session cookies.
                    .allow_credentials(true)
                    // Instruct the browser to allow JavaScript on the configured origin
                    // to read the `CsrfSynchronizerTokenLayer` default response header.
                    .expose_headers(["X-CSRF-TOKEN"
                        .parse()
                        .wrap_err("Failed to parse token header.")?]),
            );

        serve(app, 4000).await?;

        Ok::<(), eyre::Report>(())
    };

    tokio::try_join!(frontend, backend)?;

    Ok(())
}

async fn serve(app: Router, port: u16) -> eyre::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    Server::try_bind(&addr)
        .wrap_err("Could not bind to network address.")?
        .serve(app.into_make_service())
        .await
        .wrap_err("Failed to serve the app.")?;

    Ok(())
}

async fn index() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html")],
        include_str!("./index.html"),
    )
}

async fn get_token() -> StatusCode {
    StatusCode::OK
}

async fn post_handler() -> StatusCode {
    StatusCode::ACCEPTED
}

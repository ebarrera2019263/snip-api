//! snip-api: a URL shortener REST API built with Axum, SQLx and SQLite.

pub mod auth;
pub mod config;
pub mod db;
pub mod demo;
pub mod docs;
pub mod error;
pub mod extract;
pub mod models;
pub mod routes;
pub mod state;
mod validation;

use axum::{
    Json, Router,
    routing::{get, post},
};
use std::path::Path;

use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::{docs::ApiDoc, state::AppState};

/// Serves the built React dashboard from `dir` (the output of `npm run build`).
/// Its client-side pages all get `index.html`; `/{code}` keeps handling short links.
fn with_dashboard(router: Router<AppState>, dir: &Path) -> Router<AppState> {
    let index = ServeFile::new(dir.join("index.html"));
    router
        .route_service("/", index.clone())
        .route_service("/links/{id}", index)
        .route_service("/favicon.svg", ServeFile::new(dir.join("favicon.svg")))
        .nest_service("/assets", ServeDir::new(dir.join("assets")))
}

pub fn app(state: AppState) -> Router {
    let api = Router::new()
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/login", post(routes::auth::login))
        .route("/auth/me", get(routes::auth::me))
        .route(
            "/links",
            get(routes::links::list).post(routes::links::create),
        )
        .route(
            "/links/{id}",
            get(routes::links::get_one)
                .patch(routes::links::update)
                .delete(routes::links::delete),
        )
        .route("/links/{id}/stats", get(routes::links::stats))
        .route("/demo", get(routes::demo_account));

    let mut router = Router::new()
        .nest("/api", api)
        .route("/health", get(routes::health))
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        // Static routes above take priority over this catch-all.
        .route("/{code}", get(routes::redirect::follow));

    if let Some(dir) = &state.config.static_dir {
        router = with_dashboard(router, Path::new(dir));
    }

    router
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

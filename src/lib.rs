//! snip-api: a URL shortener REST API built with Axum, SQLx and SQLite.

pub mod auth;
pub mod config;
pub mod db;
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
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::{docs::ApiDoc, state::AppState};

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
        .route("/links/{id}/stats", get(routes::links::stats));

    Router::new()
        .nest("/api", api)
        .route("/health", get(routes::health))
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        // Static routes above take priority over this catch-all.
        .route("/{code}", get(routes::redirect::follow))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

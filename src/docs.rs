use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::{error, models, routes};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "snip-api",
        description = "URL shortener with accounts, expiring links and click analytics."
    ),
    paths(
        routes::health,
        routes::auth::register,
        routes::auth::login,
        routes::auth::me,
        routes::links::create,
        routes::links::list,
        routes::links::get_one,
        routes::links::update,
        routes::links::delete,
        routes::links::stats,
        routes::redirect::follow,
    ),
    components(schemas(error::ErrorBody, models::Link, models::LinkPage, models::LinkStats)),
    modifiers(&BearerAuth),
    tags(
        (name = "auth", description = "Accounts and tokens"),
        (name = "links", description = "Manage short links (requires a token)"),
        (name = "redirect", description = "Public short-link redirects"),
        (name = "health", description = "Service status"),
    )
)]
pub struct ApiDoc;

/// Registers the `bearer` security scheme referenced by protected routes.
struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

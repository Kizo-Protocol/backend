use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Kizo Prediction Market API",
        version = "1.0.0",
        description = "REST API for Kizo Prediction Market platform on Aptos blockchain",
        contact(
            name = "Kizo Team",
            email = "team@kizo.io"
        ),
        license(
            name = "Apache-2.0"
        )
    ),
    servers(
        (url = "http://localhost:3002", description = "Local development server"),
        (url = "https://api.kizo.io", description = "Production server")
    ),
    paths(

        crate::routes::health_check,


        crate::routes::markets::get_markets,
        crate::routes::markets::get_market_by_identifier,
        crate::routes::markets::get_platform_stats,


        crate::routes::protocols::get_bets_with_filters,


        crate::routes::protocols::get_protocols,
        crate::routes::protocols::get_protocol_by_id,


        crate::routes::yields::get_yields,
        crate::routes::yields::get_yield_protocols,


        crate::routes::charts::get_market_chart,
        crate::routes::charts::get_market_probability,
        crate::routes::charts::get_market_volume,
        crate::routes::charts::get_chart_config,
    ),
    components(
        schemas(

            crate::models::Market,
            crate::models::Bet,
            crate::models::Protocol,
            crate::models::YieldRecord,
            crate::models::PaginationParams,
            crate::routes::protocols::BetFilters,
            crate::routes::protocols::PlaceBetRequest,


            crate::models::MarketStats,
            crate::models::PlatformStats,
            crate::models::UserStats,
        )
    ),
    tags(
        (name = "markets", description = "Market management endpoints"),
        (name = "bets", description = "Betting operations"),
        (name = "protocols", description = "Yield protocol management"),
        (name = "yields", description = "Yield tracking and statistics"),
        (name = "charts", description = "Chart data for market visualization"),
        (name = "health", description = "Health check and status")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

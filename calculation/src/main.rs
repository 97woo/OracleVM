use axum::{extract::Query, http::StatusCode, response::Json, routing::get, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

mod models;
mod pricing;
mod repositories;
mod services;
mod price_updater;

use models::{DeltaInfo, MarketState, OptionPremium, PremiumQuery};
use pricing::BlackScholesPricing;
use repositories::{InMemoryMarketRepo, InMemoryPoolRepo, InMemoryPremiumRepo};
use services::{DeltaManagementService, MarketDataService, PremiumCalculationService};
use price_updater::PriceUpdater;

/// 애플리케이션 상태
struct AppState {
    premium_service: Arc<PremiumCalculationService<BlackScholesPricing>>,
    delta_service: Arc<DeltaManagementService>,
    market_service: Arc<MarketDataService>,
}

async fn get_premium_map(
    Query(params): Query<PremiumQuery>,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Result<Json<Vec<OptionPremium>>, StatusCode> {
    match state.premium_service.get_premiums_by_expiry(params.expiry).await {
        Ok(premiums) => Ok(Json(premiums)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_pool_delta(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Result<Json<DeltaInfo>, StatusCode> {
    match state.delta_service.get_pool_delta().await {
        Ok(delta_info) => Ok(Json(delta_info)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_current_delta(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Result<Json<f64>, StatusCode> {
    match state.delta_service.get_current_delta().await {
        Ok(delta) => Ok(Json(delta)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_market_state(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Result<Json<MarketState>, StatusCode> {
    match state.market_service.get_market_state().await {
        Ok(market_state) => Ok(Json(market_state)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // 저장소 초기화
    let premium_repo = Arc::new(InMemoryPremiumRepo::new());
    let pool_repo = Arc::new(InMemoryPoolRepo::new());
    let market_repo = Arc::new(InMemoryMarketRepo::new());

    // 서비스 초기화
    let pricing_engine = BlackScholesPricing::new();
    let premium_service = Arc::new(PremiumCalculationService::new(
        pricing_engine,
        premium_repo.clone(),
        market_repo.clone(),
    ));
    let delta_service = Arc::new(DeltaManagementService::new(pool_repo.clone()));
    let market_service = Arc::new(MarketDataService::new(market_repo.clone()));

    // 초기 데이터 설정
    premium_service.update_premium_map(70000.0).await.unwrap();

    // Oracle Aggregator와 연동하여 실시간 가격 업데이트 시작
    let aggregator_url = std::env::var("AGGREGATOR_URL")
        .unwrap_or_else(|_| "http://localhost:50051".to_string());
    
    info!("Connecting to Oracle Aggregator at {}", aggregator_url);
    
    let price_updater = PriceUpdater::new(
        premium_service.clone(),
        aggregator_url,
    );
    
    // 백그라운드에서 가격 업데이트 실행
    let updater_handle = tokio::spawn(async move {
        if let Err(e) = price_updater.start().await {
            tracing::error!("Price updater error: {}", e);
        }
    });

    // 애플리케이션 상태
    let app_state = Arc::new(AppState {
        premium_service,
        delta_service,
        market_service,
    });

    let app = Router::new()
        .route("/api/premium", get(get_premium_map))
        .route("/api/pool/delta", get(get_pool_delta))
        .route("/api/delta/current", get(get_current_delta))
        .route("/api/market", get(get_market_state))
        .with_state(app_state);

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to address");

    info!("Calculation API server starting on http://127.0.0.1:3000");
    info!("Available endpoints:");
    info!("  GET /api/premium - 프리미엄 맵");
    info!("  GET /api/pool/delta - 풀 델타 정보");
    info!("  GET /api/delta/current - 현재 델타값");
    info!("  GET /api/market - 시장 상태");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OptionParameters;
    use crate::pricing::PricingEngine;

    #[tokio::test]
    async fn test_api_integration() {
        // 저장소 초기화
        let premium_repo = Arc::new(InMemoryPremiumRepo::new());
        let pool_repo = Arc::new(InMemoryPoolRepo::new());
        let market_repo = Arc::new(InMemoryMarketRepo::new());

        // 서비스 초기화
        let pricing_engine = BlackScholesPricing::new();
        let premium_service = Arc::new(PremiumCalculationService::new(
            pricing_engine,
            premium_repo.clone(),
            market_repo.clone(),
        ));

        // 프리미엄 업데이트
        premium_service.update_premium_map(70000.0).await.unwrap();

        // 조회 테스트
        let premiums = premium_service
            .get_premiums_by_expiry(Some("2024-02-01".to_string()))
            .await
            .unwrap();

        assert!(!premiums.is_empty());
        assert_eq!(premiums[0].expiry, "2024-02-01");
    }

    #[test]
    fn test_pricing_engine() {
        let pricing = BlackScholesPricing::new();
        
        let params = OptionParameters {
            spot: 70000.0,
            strike: 70000.0,
            time_to_expiry: 30.0 / 365.0,
            volatility: 0.6,
            risk_free_rate: 0.05,
            is_call: true,
        };

        let price = pricing.calculate_option_price(&params);
        let delta = pricing.calculate_delta(&params);

        assert!(price > 0.0);
        assert!(price < 70000.0);
        assert!(delta > 0.4 && delta < 0.6);
    }
}

use crate::models::{DeltaInfo, MarketState, OptionParameters, OptionPremium};
use crate::pricing::{calculate_time_to_expiry, PricingEngine};
use crate::repositories::{MarketDataRepository, PoolStateRepository, PremiumRepository};
use std::sync::Arc;

/// 프리미엄 계산 서비스
pub struct PremiumCalculationService<P> {
    pricing_engine: P,
    premium_repo: Arc<dyn PremiumRepository>,
    market_repo: Arc<dyn MarketDataRepository>,
}

impl<P> PremiumCalculationService<P>
where
    P: PricingEngine,
{
    pub fn new(
        pricing_engine: P,
        premium_repo: Arc<dyn PremiumRepository>,
        market_repo: Arc<dyn MarketDataRepository>,
    ) -> Self {
        Self {
            pricing_engine,
            premium_repo,
            market_repo,
        }
    }

    /// 프리미엄 맵 업데이트
    pub async fn update_premium_map(&self, current_price: f64) -> Result<(), String> {
        let strikes = vec![60000.0, 65000.0, 70000.0, 75000.0, 80000.0];
        let expiries = vec!["2024-02-01", "2024-03-01", "2024-04-01"];
        let risk_free_rate = 0.05;

        let market_state = self.market_repo.get_current_state().await?;

        for expiry in &expiries {
            let mut options = Vec::new();
            let time_to_expiry = calculate_time_to_expiry(expiry);

            for &strike in &strikes {
                let call_params = OptionParameters {
                    spot: current_price,
                    strike,
                    time_to_expiry,
                    volatility: market_state.volatility_24h,
                    risk_free_rate,
                    is_call: true,
                };

                let put_params = OptionParameters {
                    spot: current_price,
                    strike,
                    time_to_expiry,
                    volatility: market_state.volatility_24h,
                    risk_free_rate,
                    is_call: false,
                };

                let call_premium = self.pricing_engine.calculate_option_price(&call_params);
                let put_premium = self.pricing_engine.calculate_option_price(&put_params);

                options.push(OptionPremium {
                    strike,
                    expiry: expiry.to_string(),
                    call_premium,
                    put_premium,
                    implied_volatility: market_state.volatility_24h,
                });
            }

            self.premium_repo
                .save_premiums(expiry.to_string(), options)
                .await?;
        }

        Ok(())
    }

    /// 특정 만기의 프리미엄 조회
    pub async fn get_premiums_by_expiry(
        &self,
        expiry: Option<String>,
    ) -> Result<Vec<OptionPremium>, String> {
        if let Some(exp) = expiry {
            self.premium_repo.get_premiums_by_expiry(&exp).await
        } else {
            self.premium_repo.get_all_premiums().await
        }
    }
}

/// 델타 관리 서비스
pub struct DeltaManagementService {
    pool_repo: Arc<dyn PoolStateRepository>,
}

impl DeltaManagementService {
    pub fn new(pool_repo: Arc<dyn PoolStateRepository>) -> Self {
        Self {
            pool_repo,
        }
    }

    /// 풀 델타 정보 조회
    pub async fn get_pool_delta(&self) -> Result<DeltaInfo, String> {
        self.pool_repo.get_delta_info().await
    }

    /// 현재 네트 델타 조회
    pub async fn get_current_delta(&self) -> Result<f64, String> {
        let delta_info = self.pool_repo.get_delta_info().await?;
        Ok(delta_info.net_delta)
    }

    /// 새로운 포지션 추가
    pub async fn update_pool_position(
        &self,
        delta: f64,
        is_call: bool,
    ) -> Result<(), String> {
        let mut delta_info = self.pool_repo.get_delta_info().await?;
        delta_info.add_delta(delta, is_call);
        self.pool_repo.update_delta_info(delta_info).await
    }
}

/// 시장 데이터 서비스
pub struct MarketDataService {
    market_repo: Arc<dyn MarketDataRepository>,
}

impl MarketDataService {
    pub fn new(market_repo: Arc<dyn MarketDataRepository>) -> Self {
        Self { market_repo }
    }

    /// 현재 시장 상태 조회
    pub async fn get_market_state(&self) -> Result<MarketState, String> {
        self.market_repo.get_current_state().await
    }

    /// 시장 상태 업데이트
    pub async fn update_market_state(&self, state: MarketState) -> Result<(), String> {
        self.market_repo.update_state(state).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::BlackScholesPricing;
    use crate::repositories::{InMemoryMarketRepo, InMemoryPoolRepo, InMemoryPremiumRepo};

    #[tokio::test]
    async fn test_premium_calculation_service() {
        let pricing_engine = BlackScholesPricing::new();
        let premium_repo = Arc::new(InMemoryPremiumRepo::new());
        let market_repo = Arc::new(InMemoryMarketRepo::new());

        let service = PremiumCalculationService::new(
            pricing_engine,
            premium_repo.clone(),
            market_repo.clone(),
        );

        service.update_premium_map(70000.0).await.unwrap();

        let premiums = service
            .get_premiums_by_expiry(Some("2024-02-01".to_string()))
            .await
            .unwrap();

        assert!(!premiums.is_empty());
    }

    #[tokio::test]
    async fn test_delta_management_service() {
        let pool_repo = Arc::new(InMemoryPoolRepo::new());
        let service = DeltaManagementService::new(pool_repo.clone());

        let delta = service.get_current_delta().await.unwrap();
        assert_eq!(delta, 0.0);

        service.update_pool_position(0.5, true).await.unwrap();
        
        let updated_delta = service.get_current_delta().await.unwrap();
        assert_eq!(updated_delta, 0.5);
    }
}
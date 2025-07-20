use crate::models::{DeltaInfo, MarketState, OptionPremium};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// 프리미엄 저장소 인터페이스
#[async_trait]
pub trait PremiumRepository: Send + Sync {
    async fn save_premiums(&self, expiry: String, premiums: Vec<OptionPremium>) -> Result<(), String>;
    async fn get_premiums_by_expiry(&self, expiry: &str) -> Result<Vec<OptionPremium>, String>;
    async fn get_all_premiums(&self) -> Result<Vec<OptionPremium>, String>;
    async fn clear(&self) -> Result<(), String>;
}

/// 풀 상태 저장소 인터페이스
#[async_trait]
pub trait PoolStateRepository: Send + Sync {
    async fn get_delta_info(&self) -> Result<DeltaInfo, String>;
    async fn update_delta_info(&self, delta_info: DeltaInfo) -> Result<(), String>;
}

/// 시장 데이터 저장소 인터페이스
#[async_trait]
pub trait MarketDataRepository: Send + Sync {
    async fn get_current_state(&self) -> Result<MarketState, String>;
    async fn update_state(&self, state: MarketState) -> Result<(), String>;
}

/// 인메모리 프리미엄 저장소 구현
pub struct InMemoryPremiumRepo {
    data: RwLock<HashMap<String, Vec<OptionPremium>>>,
}

impl InMemoryPremiumRepo {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryPremiumRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PremiumRepository for InMemoryPremiumRepo {
    async fn save_premiums(&self, expiry: String, premiums: Vec<OptionPremium>) -> Result<(), String> {
        let mut data = self.data.write().map_err(|_| "Lock error")?;
        data.insert(expiry, premiums);
        Ok(())
    }

    async fn get_premiums_by_expiry(&self, expiry: &str) -> Result<Vec<OptionPremium>, String> {
        let data = self.data.read().map_err(|_| "Lock error")?;
        data.get(expiry)
            .cloned()
            .ok_or_else(|| "Premiums not found".to_string())
    }

    async fn get_all_premiums(&self) -> Result<Vec<OptionPremium>, String> {
        let data = self.data.read().map_err(|_| "Lock error")?;
        Ok(data.values().flat_map(|v| v.clone()).collect())
    }

    async fn clear(&self) -> Result<(), String> {
        let mut data = self.data.write().map_err(|_| "Lock error")?;
        data.clear();
        Ok(())
    }
}

/// 인메모리 풀 상태 저장소 구현
pub struct InMemoryPoolRepo {
    delta_info: RwLock<DeltaInfo>,
}

impl InMemoryPoolRepo {
    pub fn new() -> Self {
        Self {
            delta_info: RwLock::new(DeltaInfo::new(1000000.0)),
        }
    }
}

impl Default for InMemoryPoolRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PoolStateRepository for InMemoryPoolRepo {
    async fn get_delta_info(&self) -> Result<DeltaInfo, String> {
        let info = self.delta_info.read().map_err(|_| "Lock error")?;
        Ok(info.clone())
    }

    async fn update_delta_info(&self, delta_info: DeltaInfo) -> Result<(), String> {
        let mut info = self.delta_info.write().map_err(|_| "Lock error")?;
        *info = delta_info;
        Ok(())
    }
}

/// 인메모리 시장 데이터 저장소 구현
pub struct InMemoryMarketRepo {
    state: RwLock<MarketState>,
}

impl InMemoryMarketRepo {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(MarketState::new(70000.0, 0.6)),
        }
    }
}

impl Default for InMemoryMarketRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketDataRepository for InMemoryMarketRepo {
    async fn get_current_state(&self) -> Result<MarketState, String> {
        let state = self.state.read().map_err(|_| "Lock error")?;
        Ok(state.clone())
    }

    async fn update_state(&self, state: MarketState) -> Result<(), String> {
        let mut current = self.state.write().map_err(|_| "Lock error")?;
        *current = state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_premium_repository() {
        let repo = InMemoryPremiumRepo::new();
        
        let premiums = vec![
            OptionPremium {
                strike: 70000.0,
                expiry: "2024-02-01".to_string(),
                call_premium: 2500.0,
                put_premium: 1800.0,
                implied_volatility: 0.6,
            },
        ];

        repo.save_premiums("2024-02-01".to_string(), premiums.clone())
            .await
            .unwrap();

        let retrieved = repo.get_premiums_by_expiry("2024-02-01").await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].strike, 70000.0);
    }

    #[tokio::test]
    async fn test_pool_repository() {
        let repo = InMemoryPoolRepo::new();
        
        let mut delta_info = repo.get_delta_info().await.unwrap();
        assert_eq!(delta_info.net_delta, 0.0);

        delta_info.add_delta(0.5, true);
        repo.update_delta_info(delta_info.clone()).await.unwrap();

        let updated = repo.get_delta_info().await.unwrap();
        assert_eq!(updated.total_call_delta, 0.5);
        assert_eq!(updated.net_delta, 0.5);
    }
}
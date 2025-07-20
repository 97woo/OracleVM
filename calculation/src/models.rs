use serde::{Deserialize, Serialize};

/// 옵션 프리미엄 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionPremium {
    pub strike: f64,
    pub expiry: String,
    pub call_premium: f64,
    pub put_premium: f64,
    pub implied_volatility: f64,
}

/// 델타 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaInfo {
    pub total_call_delta: f64,
    pub total_put_delta: f64,
    pub net_delta: f64,
    pub available_liquidity: f64,
}

impl DeltaInfo {
    pub fn new(available_liquidity: f64) -> Self {
        Self {
            total_call_delta: 0.0,
            total_put_delta: 0.0,
            net_delta: 0.0,
            available_liquidity,
        }
    }

    pub fn add_delta(&mut self, delta: f64, is_call: bool) {
        if is_call {
            self.total_call_delta += delta;
        } else {
            self.total_put_delta += delta;
        }
        self.net_delta = self.total_call_delta + self.total_put_delta;
    }
}

/// 현재 시장 상태
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub current_price: f64,
    pub timestamp: u64,
    pub volatility_24h: f64,
    pub total_volume: f64,
}

impl MarketState {
    pub fn new(current_price: f64, volatility: f64) -> Self {
        Self {
            current_price,
            timestamp: 0,
            volatility_24h: volatility,
            total_volume: 0.0,
        }
    }
}

/// 옵션 파라미터
#[derive(Debug, Clone)]
pub struct OptionParameters {
    pub spot: f64,
    pub strike: f64,
    pub time_to_expiry: f64,
    pub volatility: f64,
    pub risk_free_rate: f64,
    pub is_call: bool,
}

/// API 쿼리 파라미터
#[derive(Deserialize)]
pub struct PremiumQuery {
    pub expiry: Option<String>,
}
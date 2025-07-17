use axum::{extract::Query, http::StatusCode, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::net::TcpListener;
use tracing::info;

/// 옵션 프리미엄 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionPremium {
    pub strike: f64,
    pub expiry: String, // ISO 8601 format
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

/// 현재 시장 상태
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub current_price: f64,
    pub timestamp: u64,
    pub volatility_24h: f64,
    pub total_volume: f64,
}

/// 계산 모듈 서버
pub struct CalculationServer {
    premium_map: HashMap<String, Vec<OptionPremium>>,
    pool_state: DeltaInfo,
    market_state: MarketState,
}

impl CalculationServer {
    pub fn new() -> Self {
        Self {
            premium_map: HashMap::new(),
            pool_state: DeltaInfo {
                total_call_delta: 0.0,
                total_put_delta: 0.0,
                net_delta: 0.0,
                available_liquidity: 1000000.0, // 초기 유동성 $1M
            },
            market_state: MarketState {
                current_price: 70000.0,
                timestamp: 0,
                volatility_24h: 0.6, // 60% 연간 변동성
                total_volume: 0.0,
            },
        }
    }

    /// Black-Scholes 옵션 가격 계산
    fn calculate_black_scholes(
        &self,
        spot: f64,
        strike: f64,
        time_to_expiry: f64,
        volatility: f64,
        risk_free_rate: f64,
        is_call: bool,
    ) -> f64 {
        if time_to_expiry <= 0.0 {
            return if is_call {
                (spot - strike).max(0.0)
            } else {
                (strike - spot).max(0.0)
            };
        }

        let d1 = ((spot / strike).ln()
            + (risk_free_rate + volatility.powi(2) / 2.0) * time_to_expiry)
            / (volatility * time_to_expiry.sqrt());
        let d2 = d1 - volatility * time_to_expiry.sqrt();

        let n_d1 = normal_cdf(d1);
        let n_d2 = normal_cdf(d2);
        let n_neg_d1 = normal_cdf(-d1);
        let n_neg_d2 = normal_cdf(-d2);

        if is_call {
            spot * n_d1 - strike * (-risk_free_rate * time_to_expiry).exp() * n_d2
        } else {
            strike * (-risk_free_rate * time_to_expiry).exp() * n_neg_d2 - spot * n_neg_d1
        }
    }

    /// 델타 계산
    #[allow(dead_code)]
    fn calculate_delta(
        &self,
        spot: f64,
        strike: f64,
        time_to_expiry: f64,
        volatility: f64,
        risk_free_rate: f64,
        is_call: bool,
    ) -> f64 {
        if time_to_expiry <= 0.0 {
            return if is_call {
                if spot > strike {
                    1.0
                } else {
                    0.0
                }
            } else if spot < strike {
                -1.0
            } else {
                0.0
            };
        }

        let d1 = ((spot / strike).ln()
            + (risk_free_rate + volatility.powi(2) / 2.0) * time_to_expiry)
            / (volatility * time_to_expiry.sqrt());

        if is_call {
            normal_cdf(d1)
        } else {
            normal_cdf(d1) - 1.0
        }
    }

}

impl Default for CalculationServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculationServer {
    /// 프리미엄 맵 업데이트
    pub fn update_premium_map(&mut self, current_price: f64) {
        let strikes = vec![60000.0, 65000.0, 70000.0, 75000.0, 80000.0];
        let expiries = vec!["2024-02-01", "2024-03-01", "2024-04-01"];
        let risk_free_rate = 0.05; // 5% 무위험 수익률

        self.premium_map.clear();

        for expiry in &expiries {
            let mut options = Vec::new();
            let time_to_expiry = calculate_time_to_expiry(expiry); // 년 단위

            for &strike in &strikes {
                let call_premium = self.calculate_black_scholes(
                    current_price,
                    strike,
                    time_to_expiry,
                    self.market_state.volatility_24h,
                    risk_free_rate,
                    true,
                );

                let put_premium = self.calculate_black_scholes(
                    current_price,
                    strike,
                    time_to_expiry,
                    self.market_state.volatility_24h,
                    risk_free_rate,
                    false,
                );

                options.push(OptionPremium {
                    strike,
                    expiry: expiry.to_string(),
                    call_premium,
                    put_premium,
                    implied_volatility: self.market_state.volatility_24h,
                });
            }

            self.premium_map.insert(expiry.to_string(), options);
        }

        info!("Premium map updated for price: ${:.2}", current_price);
    }

    /// 풀 상태 업데이트
    pub fn update_pool_state(&mut self, new_position: f64, is_call: bool) {
        if is_call {
            self.pool_state.total_call_delta += new_position;
        } else {
            self.pool_state.total_put_delta += new_position;
        }

        self.pool_state.net_delta =
            self.pool_state.total_call_delta + self.pool_state.total_put_delta;

        info!(
            "Pool state updated - Net Delta: {:.4}",
            self.pool_state.net_delta
        );
    }
}

/// 표준정규분포 누적밀도함수 근사
fn normal_cdf(x: f64) -> f64 {
    (1.0 + libm::erf(x / 2.0f64.sqrt())) / 2.0
}

/// 만료일까지 시간 계산 (년 단위)
fn calculate_time_to_expiry(expiry: &str) -> f64 {
    // 간단한 구현 - 실제로는 정확한 날짜 계산 필요
    match expiry {
        "2024-02-01" => 30.0 / 365.0, // 약 1개월
        "2024-03-01" => 60.0 / 365.0, // 약 2개월
        "2024-04-01" => 90.0 / 365.0, // 약 3개월
        _ => 30.0 / 365.0,
    }
}

/// API 엔드포인트들
#[derive(Deserialize)]
struct PremiumQuery {
    expiry: Option<String>,
}

async fn get_premium_map(
    Query(params): Query<PremiumQuery>,
) -> Result<Json<HashMap<String, Vec<OptionPremium>>>, StatusCode> {
    // 임시 데이터 - 실제로는 전역 상태에서 가져와야 함
    let mut premium_map = HashMap::new();

    let options = vec![OptionPremium {
        strike: 70000.0,
        expiry: "2024-02-01".to_string(),
        call_premium: 2500.0,
        put_premium: 1800.0,
        implied_volatility: 0.6,
    }];

    premium_map.insert("2024-02-01".to_string(), options);

    if let Some(expiry) = params.expiry {
        if let Some(options) = premium_map.get(&expiry) {
            let mut filtered = HashMap::new();
            filtered.insert(expiry, options.clone());
            return Ok(Json(filtered));
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    Ok(Json(premium_map))
}

async fn get_pool_delta() -> Json<DeltaInfo> {
    Json(DeltaInfo {
        total_call_delta: 150.5,
        total_put_delta: -230.8,
        net_delta: -80.3,
        available_liquidity: 950000.0,
    })
}

async fn get_current_delta() -> Json<f64> {
    Json(-80.3)
}

async fn get_market_state() -> Json<MarketState> {
    Json(MarketState {
        current_price: 71250.0,
        timestamp: 1700000000,
        volatility_24h: 0.65,
        total_volume: 12500000.0,
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/api/premium", get(get_premium_map))
        .route("/api/pool/delta", get(get_pool_delta))
        .route("/api/delta/current", get(get_current_delta))
        .route("/api/market", get(get_market_state));

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

    #[test]
    fn test_black_scholes_calculation() {
        let calc = CalculationServer::new();

        // ATM Call 옵션 테스트
        let call_price = calc.calculate_black_scholes(
            70000.0,      // spot
            70000.0,      // strike
            30.0 / 365.0, // 30일
            0.6,          // 60% 변동성
            0.05,         // 5% 무위험 수익률
            true,         // call 옵션
        );

        assert!(call_price > 0.0);
        assert!(call_price < 70000.0); // 프리미엄이 spot보다 작아야 함

        println!("ATM Call Premium: ${:.2}", call_price);
    }

    #[test]
    fn test_delta_calculation() {
        let calc = CalculationServer::new();

        // ATM Call 델타는 약 0.5여야 함
        let call_delta = calc.calculate_delta(70000.0, 70000.0, 30.0 / 365.0, 0.6, 0.05, true);

        assert!(call_delta > 0.4 && call_delta < 0.6);
        println!("ATM Call Delta: {:.4}", call_delta);
    }
}

use crate::models::OptionParameters;
use crate::pricing::{BlackScholesPricing, PricingEngine};

/// Target Theta 기반 옵션 프리미엄 계산
pub struct ThetaTargetingEngine {
    pricing_engine: BlackScholesPricing,
}

impl ThetaTargetingEngine {
    pub fn new() -> Self {
        Self {
            pricing_engine: BlackScholesPricing::new(),
        }
    }

    /// Target theta를 달성하기 위한 implied volatility 찾기
    pub fn find_iv_for_target_theta(
        &self,
        spot: f64,
        strike: f64,
        time_to_expiry: f64,
        risk_free_rate: f64,
        is_call: bool,
        target_theta: f64, // 일일 theta (음수)
    ) -> Result<f64, String> {
        // Newton-Raphson method로 IV 찾기
        let mut iv = 0.5; // 초기 추정값 50%
        let tolerance = 0.0001;
        let max_iterations = 100;
        
        for _ in 0..max_iterations {
            let params = OptionParameters {
                spot,
                strike,
                volatility: iv,
                risk_free_rate,
                time_to_expiry,
                is_call,
            };
            
            let current_theta = self.pricing_engine.calculate_theta(&params);
            let daily_theta = current_theta / 365.0; // 연간 theta를 일일 theta로 변환
            
            let diff = daily_theta - target_theta;
            
            if diff.abs() < tolerance {
                return Ok(iv);
            }
            
            // Vega를 사용해 다음 IV 추정
            let vega = self.pricing_engine.calculate_vega(&params);
            if vega.abs() < 1e-10 {
                return Err("Vega too small for convergence".to_string());
            }
            
            // Theta는 IV에 대해 증가함수이므로
            iv = iv - diff / (vega * 0.01); // 적절한 스케일링
            
            // IV 범위 제한
            iv = iv.max(0.01).min(5.0); // 1% ~ 500%
        }
        
        Err("Failed to converge to target theta".to_string())
    }

    /// 3개 거래소 평균 가격을 사용한 프리미엄 계산
    pub fn calculate_premium_with_target_theta(
        &self,
        binance_price: f64,
        coinbase_price: f64,
        kraken_price: f64,
        strike: f64,
        time_to_expiry_days: f64,
        risk_free_rate: f64,
        is_call: bool,
        target_theta: f64,
        notional_btc: f64, // BTC 단위 수량
    ) -> Result<PremiumResult, String> {
        // 3개 거래소 평균 가격
        let spot = (binance_price + coinbase_price + kraken_price) / 3.0;
        
        // 연 단위로 변환
        let time_to_expiry = time_to_expiry_days / 365.0;
        
        // Target theta에 맞는 IV 찾기
        let implied_vol = self.find_iv_for_target_theta(
            spot,
            strike,
            time_to_expiry,
            risk_free_rate,
            is_call,
            target_theta,
        )?;
        
        // 옵션 가격 계산
        let params = OptionParameters {
            spot,
            strike,
            volatility: implied_vol,
            risk_free_rate,
            time_to_expiry,
            is_call,
        };
        
        let option_price = self.pricing_engine.calculate_option_price(&params);
        let delta = self.pricing_engine.calculate_delta(&params);
        let gamma = self.pricing_engine.calculate_gamma(&params);
        let vega = self.pricing_engine.calculate_vega(&params);
        let theta = self.pricing_engine.calculate_theta(&params);
        let rho = self.pricing_engine.calculate_rho(&params);
        
        // BTC 단위로 프리미엄 계산
        let premium_btc = (option_price / spot) * notional_btc;
        
        Ok(PremiumResult {
            spot_price: spot,
            strike_price: strike,
            implied_volatility: implied_vol,
            premium_usd: option_price * notional_btc,
            premium_btc,
            delta: delta * notional_btc,
            gamma: gamma * notional_btc,
            vega: vega * notional_btc,
            theta: theta * notional_btc,
            daily_theta: (theta / 365.0) * notional_btc,
            rho: rho * notional_btc,
        })
    }
}

/// 프리미엄 계산 결과
#[derive(Debug, Clone)]
pub struct PremiumResult {
    pub spot_price: f64,
    pub strike_price: f64,
    pub implied_volatility: f64,
    pub premium_usd: f64,
    pub premium_btc: f64,
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub daily_theta: f64,
    pub rho: f64,
}

/// Delta-neutral 포트폴리오 관리
pub struct DeltaNeutralManager {
    engine: ThetaTargetingEngine,
}

impl DeltaNeutralManager {
    pub fn new() -> Self {
        Self {
            engine: ThetaTargetingEngine::new(),
        }
    }

    /// 포트폴리오의 총 델타 계산
    pub fn calculate_portfolio_delta(
        &self,
        positions: &[OptionPosition],
        spot_price: f64,
    ) -> f64 {
        positions.iter()
            .map(|pos| {
                let params = OptionParameters {
                    spot: spot_price,
                    strike: pos.strike,
                    volatility: pos.implied_vol,
                    risk_free_rate: 0.05,
                    time_to_expiry: pos.days_to_expiry / 365.0,
                    is_call: pos.is_call,
                };
                
                let delta = self.engine.pricing_engine.calculate_delta(&params);
                delta * pos.quantity * if pos.is_long { 1.0 } else { -1.0 }
            })
            .sum()
    }

    /// 델타 중립을 위한 헷지 수량 계산
    pub fn calculate_hedge_amount(&self, portfolio_delta: f64) -> f64 {
        -portfolio_delta // 반대 포지션으로 헷지
    }

    /// 포트폴리오의 총 세타 수익 계산
    pub fn calculate_portfolio_theta_revenue(
        &self,
        positions: &[OptionPosition],
        spot_price: f64,
    ) -> f64 {
        positions.iter()
            .map(|pos| {
                let params = OptionParameters {
                    spot: spot_price,
                    strike: pos.strike,
                    volatility: pos.implied_vol,
                    risk_free_rate: 0.05,
                    time_to_expiry: pos.days_to_expiry / 365.0,
                    is_call: pos.is_call,
                };
                
                let theta = self.engine.pricing_engine.calculate_theta(&params);
                let daily_theta = theta / 365.0;
                daily_theta * pos.quantity * if pos.is_long { -1.0 } else { 1.0 }
            })
            .sum()
    }
}

/// 옵션 포지션 정보
#[derive(Debug, Clone)]
pub struct OptionPosition {
    pub strike: f64,
    pub days_to_expiry: f64,
    pub implied_vol: f64,
    pub is_call: bool,
    pub is_long: bool,
    pub quantity: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_iv_for_target_theta() {
        let engine = ThetaTargetingEngine::new();
        
        // Target theta = -0.02 (2% daily decay)
        let result = engine.find_iv_for_target_theta(
            70000.0, // spot
            75000.0, // strike
            7.0 / 365.0, // 7 days
            0.05, // risk-free rate
            true, // call
            -0.02, // target theta
        );
        
        assert!(result.is_ok());
        let iv = result.unwrap();
        assert!(iv > 0.0 && iv < 5.0);
    }

    #[test]
    fn test_premium_calculation_with_aggregated_prices() {
        let engine = ThetaTargetingEngine::new();
        
        let result = engine.calculate_premium_with_target_theta(
            69950.0, // Binance
            70000.0, // Coinbase
            70050.0, // Kraken
            75000.0, // Strike
            7.0,     // 7 days
            0.05,    // Risk-free rate
            true,    // Call
            -0.02,   // Target theta
            0.1,     // 0.1 BTC
        );
        
        assert!(result.is_ok());
        let premium = result.unwrap();
        assert_eq!(premium.spot_price, 70000.0);
        assert!(premium.premium_btc > 0.0);
        assert!(premium.daily_theta < 0.0);
    }

    #[test]
    fn test_delta_neutral_portfolio() {
        let manager = DeltaNeutralManager::new();
        
        let positions = vec![
            OptionPosition {
                strike: 75000.0,
                days_to_expiry: 7.0,
                implied_vol: 0.8,
                is_call: true,
                is_long: false, // 매도 포지션 (풀 입장)
                quantity: 0.1,
            },
            OptionPosition {
                strike: 65000.0,
                days_to_expiry: 7.0,
                implied_vol: 0.8,
                is_call: false,
                is_long: false, // 매도 포지션 (풀 입장)
                quantity: 0.1,
            },
        ];
        
        let portfolio_delta = manager.calculate_portfolio_delta(&positions, 70000.0);
        let hedge_amount = manager.calculate_hedge_amount(portfolio_delta);
        
        // 델타 중립을 위한 헷지 확인
        assert!((portfolio_delta + hedge_amount).abs() < 0.001);
        
        // 세타 수익 확인 (풀은 매도 포지션이므로 양수)
        let theta_revenue = manager.calculate_portfolio_theta_revenue(&positions, 70000.0);
        assert!(theta_revenue > 0.0);
    }
}
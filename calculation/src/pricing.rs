use crate::models::OptionParameters;

/// Black-Scholes 가격 계산 인터페이스
pub trait PricingEngine {
    fn calculate_option_price(&self, params: &OptionParameters) -> f64;
    fn calculate_delta(&self, params: &OptionParameters) -> f64;
    fn calculate_gamma(&self, params: &OptionParameters) -> f64;
    fn calculate_vega(&self, params: &OptionParameters) -> f64;
    fn calculate_theta(&self, params: &OptionParameters) -> f64;
    fn calculate_rho(&self, params: &OptionParameters) -> f64;
}

/// Black-Scholes 가격 계산 엔진
pub struct BlackScholesPricing;

impl BlackScholesPricing {
    pub fn new() -> Self {
        Self
    }

    /// 표준정규분포 누적밀도함수
    fn normal_cdf(&self, x: f64) -> f64 {
        (1.0 + libm::erf(x / 2.0f64.sqrt())) / 2.0
    }

    /// 표준정규분포 확률밀도함수
    fn normal_pdf(&self, x: f64) -> f64 {
        (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
    }

    /// d1 계산
    fn calculate_d1(&self, params: &OptionParameters) -> f64 {
        ((params.spot / params.strike).ln()
            + (params.risk_free_rate + params.volatility.powi(2) / 2.0) * params.time_to_expiry)
            / (params.volatility * params.time_to_expiry.sqrt())
    }

    /// d2 계산
    fn calculate_d2(&self, d1: f64, params: &OptionParameters) -> f64 {
        d1 - params.volatility * params.time_to_expiry.sqrt()
    }
}

impl Default for BlackScholesPricing {
    fn default() -> Self {
        Self::new()
    }
}

impl PricingEngine for BlackScholesPricing {
    fn calculate_option_price(&self, params: &OptionParameters) -> f64 {
        if params.time_to_expiry <= 0.0 {
            return if params.is_call {
                (params.spot - params.strike).max(0.0)
            } else {
                (params.strike - params.spot).max(0.0)
            };
        }

        let d1 = self.calculate_d1(params);
        let d2 = self.calculate_d2(d1, params);

        let n_d1 = self.normal_cdf(d1);
        let n_d2 = self.normal_cdf(d2);
        let n_neg_d1 = self.normal_cdf(-d1);
        let n_neg_d2 = self.normal_cdf(-d2);

        let discount_factor = (-params.risk_free_rate * params.time_to_expiry).exp();

        if params.is_call {
            params.spot * n_d1 - params.strike * discount_factor * n_d2
        } else {
            params.strike * discount_factor * n_neg_d2 - params.spot * n_neg_d1
        }
    }

    fn calculate_delta(&self, params: &OptionParameters) -> f64 {
        if params.time_to_expiry <= 0.0 {
            return if params.is_call {
                if params.spot > params.strike {
                    1.0
                } else {
                    0.0
                }
            } else if params.spot < params.strike {
                -1.0
            } else {
                0.0
            };
        }

        let d1 = self.calculate_d1(params);

        if params.is_call {
            self.normal_cdf(d1)
        } else {
            self.normal_cdf(d1) - 1.0
        }
    }

    fn calculate_gamma(&self, params: &OptionParameters) -> f64 {
        if params.time_to_expiry <= 0.0 {
            return 0.0;
        }

        let d1 = self.calculate_d1(params);
        let n_prime_d1 = self.normal_pdf(d1);

        n_prime_d1 / (params.spot * params.volatility * params.time_to_expiry.sqrt())
    }

    fn calculate_vega(&self, params: &OptionParameters) -> f64 {
        if params.time_to_expiry <= 0.0 {
            return 0.0;
        }

        let d1 = self.calculate_d1(params);
        let n_prime_d1 = self.normal_pdf(d1);

        params.spot * n_prime_d1 * params.time_to_expiry.sqrt() / 100.0
    }

    fn calculate_theta(&self, params: &OptionParameters) -> f64 {
        if params.time_to_expiry <= 0.0 {
            return 0.0;
        }

        let d1 = self.calculate_d1(params);
        let d2 = self.calculate_d2(d1, params);
        let n_prime_d1 = self.normal_pdf(d1);
        
        let discount_factor = (-params.risk_free_rate * params.time_to_expiry).exp();

        if params.is_call {
            let n_d2 = self.normal_cdf(d2);
            (-(params.spot * n_prime_d1 * params.volatility) / (2.0 * params.time_to_expiry.sqrt())
                - params.risk_free_rate * params.strike * discount_factor * n_d2) / 365.0
        } else {
            let n_neg_d2 = self.normal_cdf(-d2);
            (-(params.spot * n_prime_d1 * params.volatility) / (2.0 * params.time_to_expiry.sqrt())
                + params.risk_free_rate * params.strike * discount_factor * n_neg_d2) / 365.0
        }
    }

    fn calculate_rho(&self, params: &OptionParameters) -> f64 {
        if params.time_to_expiry <= 0.0 {
            return 0.0;
        }

        let d1 = self.calculate_d1(params);
        let d2 = self.calculate_d2(d1, params);
        
        let discount_factor = (-params.risk_free_rate * params.time_to_expiry).exp();

        if params.is_call {
            let n_d2 = self.normal_cdf(d2);
            params.strike * params.time_to_expiry * discount_factor * n_d2 / 100.0
        } else {
            let n_neg_d2 = self.normal_cdf(-d2);
            -params.strike * params.time_to_expiry * discount_factor * n_neg_d2 / 100.0
        }
    }
}

/// 만기일까지 시간 계산 유틸리티
pub fn calculate_time_to_expiry(expiry: &str) -> f64 {
    // 실제 구현에서는 chrono 등을 사용하여 정확한 날짜 계산
    match expiry {
        "2024-02-01" => 30.0 / 365.0,
        "2024-03-01" => 60.0 / 365.0,
        "2024-04-01" => 90.0 / 365.0,
        _ => 30.0 / 365.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_scholes_call_option() {
        let pricing = BlackScholesPricing::new();
        
        let params = OptionParameters {
            spot: 100.0,
            strike: 100.0,
            time_to_expiry: 1.0,
            volatility: 0.2,
            risk_free_rate: 0.05,
            is_call: true,
        };

        let price = pricing.calculate_option_price(&params);
        assert!(price > 0.0);
        assert!(price < params.spot);
    }

    #[test]
    fn test_greeks_calculation() {
        let pricing = BlackScholesPricing::new();
        
        let params = OptionParameters {
            spot: 100.0,
            strike: 100.0,
            time_to_expiry: 1.0,
            volatility: 0.2,
            risk_free_rate: 0.05,
            is_call: true,
        };

        let delta = pricing.calculate_delta(&params);
        assert!(delta > 0.0 && delta < 1.0);

        let gamma = pricing.calculate_gamma(&params);
        assert!(gamma > 0.0);

        let vega = pricing.calculate_vega(&params);
        assert!(vega > 0.0);
    }
}
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use oracle_vm_common::types::OptionType;

/// 단방향 옵션 (Buyer-only Option)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyerOnlyOption {
    pub option_id: String,
    pub option_type: OptionType,
    pub strike_price: u64,      // USD cents
    pub quantity: u64,          // satoshis (notional)
    pub premium_paid: u64,      // satoshis
    pub target_theta: f64,      // Target theta decay per day
    pub implied_volatility: f64, // Adjusted to match target theta
    pub expiry_timestamp: u64,   // Unix timestamp
    pub buyer_address: String,   // Bitcoin address
    pub pre_sign_tx: Vec<u8>,   // BitVMX pre-signed transaction
    pub status: OptionStatus,
}

/// 옵션 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionStatus {
    Active,
    Expired,
    Settled,
    Cancelled,
}

/// Delta-neutral 유동성 풀
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaNeutralPool {
    // 풀 기본 정보
    pub total_liquidity: u64,      // Total BTC in pool (satoshis)
    pub available_liquidity: u64,   // Available for new options
    pub locked_for_payouts: u64,    // Reserved for ITM settlements
    
    // 수익 추적
    pub total_premium_collected: u64,  // All premiums collected
    pub total_payouts: u64,            // All payouts made
    pub theta_revenue: u64,            // Revenue from theta decay
    
    // 포지션 관리
    pub net_delta: f64,           // Current net delta exposure
    pub net_gamma: f64,           // Current net gamma exposure
    pub net_vega: f64,            // Current net vega exposure
    pub net_theta: f64,           // Current net theta (daily decay)
    
    // 헷지 포지션
    pub hedge_positions: HedgePositions,
    
    // 활성 옵션
    pub active_options: HashMap<String, BuyerOnlyOption>,
}

/// 외부 거래소 헷지 포지션
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgePositions {
    pub binance_position: f64,    // BTC position on Binance
    pub bybit_position: f64,      // BTC position on Bybit
    pub total_hedge: f64,         // Total hedge position
    pub last_rebalance: u64,      // Last rebalance timestamp
}

/// 가격 데이터 (3개 거래소 평균)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedPrice {
    pub binance_price: u64,    // USD cents
    pub coinbase_price: u64,   // USD cents
    pub kraken_price: u64,     // USD cents
    pub average_price: u64,    // (binance + coinbase + kraken) / 3
    pub timestamp: u64,        // Unix timestamp
}

/// 단방향 옵션 관리자
pub struct BuyerOnlyOptionManager {
    pool: DeltaNeutralPool,
    price_cache: Option<AggregatedPrice>,
}

impl BuyerOnlyOptionManager {
    pub fn new(initial_liquidity: u64) -> Self {
        Self {
            pool: DeltaNeutralPool {
                total_liquidity: initial_liquidity,
                available_liquidity: initial_liquidity,
                locked_for_payouts: 0,
                total_premium_collected: 0,
                total_payouts: 0,
                theta_revenue: 0,
                net_delta: 0.0,
                net_gamma: 0.0,
                net_vega: 0.0,
                net_theta: 0.0,
                hedge_positions: HedgePositions {
                    binance_position: 0.0,
                    bybit_position: 0.0,
                    total_hedge: 0.0,
                    last_rebalance: 0,
                },
                active_options: HashMap::new(),
            },
            price_cache: None,
        }
    }

    /// 3개 거래소 가격 업데이트
    pub fn update_price(&mut self, aggregated_price: AggregatedPrice) {
        self.price_cache = Some(aggregated_price);
    }

    /// Target theta에 맞는 프리미엄 계산
    pub fn calculate_premium_for_target_theta(
        &self,
        option_type: OptionType,
        strike: u64,
        quantity: u64,
        target_theta: f64,
        days_to_expiry: f64,
    ) -> Result<(u64, f64)> { // Returns (premium, implied_volatility)
        let spot = self.price_cache.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No price data available"))?
            .average_price;
        
        // Simplified calculation - in production, use proper Black-Scholes
        // to find IV that gives target theta
        let base_iv = 0.8; // 80% annualized volatility
        let theta_adjustment = target_theta.abs() * 1000.0; // Simplified
        let adjusted_iv = base_iv + theta_adjustment;
        
        // Premium calculation (simplified)
        let moneyness = (spot as f64) / (strike as f64);
        let time_value = (days_to_expiry / 365.0).sqrt();
        let vol_component = adjusted_iv * time_value;
        
        let intrinsic_value = match option_type {
            OptionType::Call => ((spot as i64 - strike as i64).max(0)) as u64,
            OptionType::Put => ((strike as i64 - spot as i64).max(0)) as u64,
        };
        
        let time_value_premium = (quantity as f64 * vol_component * 0.4) as u64;
        let total_premium = intrinsic_value + time_value_premium;
        
        Ok((total_premium, adjusted_iv))
    }

    /// 옵션 구매 (단방향)
    pub fn buy_option(
        &mut self,
        option_type: OptionType,
        strike_price: u64,
        quantity: u64,
        target_theta: f64,
        days_to_expiry: f64,
        buyer_address: String,
    ) -> Result<BuyerOnlyOption> {
        // 1. Calculate premium based on target theta
        let (premium, implied_vol) = self.calculate_premium_for_target_theta(
            option_type,
            strike_price,
            quantity,
            target_theta,
            days_to_expiry,
        )?;
        
        // 2. Check available liquidity
        let spot_price = self.price_cache.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No price data available"))?
            .average_price;
            
        let max_payout = match option_type {
            OptionType::Call => quantity, // Unlimited upside
            OptionType::Put => (strike_price * quantity) / spot_price, // Limited to strike
        };
        
        if self.pool.available_liquidity < max_payout {
            anyhow::bail!("Insufficient liquidity in pool");
        }
        
        // 3. Create option
        let option_id = format!("OPT-{}-{}", 
            chrono::Utc::now().timestamp_millis(), 
            buyer_address.chars().take(8).collect::<String>()
        );
        
        let expiry_timestamp = chrono::Utc::now().timestamp() as u64 
            + (days_to_expiry * 86400.0) as u64;
        
        let option = BuyerOnlyOption {
            option_id: option_id.clone(),
            option_type,
            strike_price,
            quantity,
            premium_paid: premium,
            target_theta,
            implied_volatility: implied_vol,
            expiry_timestamp,
            buyer_address: buyer_address.clone(),
            pre_sign_tx: vec![], // Would be generated by BitVMX
            status: OptionStatus::Active,
        };
        
        // 4. Update pool state
        self.pool.available_liquidity -= max_payout;
        self.pool.locked_for_payouts += max_payout;
        self.pool.total_premium_collected += premium;
        self.pool.total_liquidity += premium;
        
        // 5. Update Greeks
        self.update_pool_greeks(&option);
        
        // 6. Store option
        self.pool.active_options.insert(option_id.clone(), option.clone());
        
        Ok(option)
    }

    /// Update pool Greeks after new option
    fn update_pool_greeks(&mut self, option: &BuyerOnlyOption) {
        // Simplified Greeks calculation
        let spot = self.price_cache.as_ref().unwrap().average_price as f64;
        let strike = option.strike_price as f64;
        let time_to_expiry = (option.expiry_timestamp - chrono::Utc::now().timestamp() as u64) as f64 / 86400.0 / 365.0;
        
        // Delta calculation (simplified)
        let moneyness = spot / strike;
        let delta = match option.option_type {
            OptionType::Call => 0.5 + 0.5 * moneyness.ln(),
            OptionType::Put => -0.5 + 0.5 * moneyness.ln(),
        }.max(-1.0).min(1.0);
        
        // Update pool Greeks
        self.pool.net_delta += delta * (option.quantity as f64 / 1e8);
        self.pool.net_theta += option.target_theta;
        
        // Trigger rebalance if delta exceeds threshold
        if self.pool.net_delta.abs() > 0.1 {
            // In production, this would trigger external hedge rebalancing
            println!("Delta rebalance needed: {}", self.pool.net_delta);
        }
    }

    /// Settle expired option
    pub fn settle_option(&mut self, option_id: &str, settlement_price: u64) -> Result<u64> {
        let option = self.pool.active_options.get_mut(option_id)
            .ok_or_else(|| anyhow::anyhow!("Option not found"))?;
        
        if option.status != OptionStatus::Active {
            anyhow::bail!("Option already settled");
        }
        
        let payout = match option.option_type {
            OptionType::Call => {
                if settlement_price > option.strike_price {
                    ((settlement_price - option.strike_price) as u64 * option.quantity) / settlement_price
                } else {
                    0
                }
            },
            OptionType::Put => {
                if settlement_price < option.strike_price {
                    ((option.strike_price - settlement_price) as u64 * option.quantity) / option.strike_price
                } else {
                    0
                }
            },
        };
        
        // Update pool state
        if payout > 0 {
            self.pool.locked_for_payouts -= payout.min(self.pool.locked_for_payouts);
            self.pool.total_payouts += payout;
            self.pool.total_liquidity = self.pool.total_liquidity.saturating_sub(payout);
        } else {
            // Option expired worthless, unlock collateral
            let locked_amount = match option.option_type {
                OptionType::Call => option.quantity,
                OptionType::Put => (option.strike_price * option.quantity) / self.price_cache.as_ref().unwrap().average_price,
            };
            self.pool.locked_for_payouts -= locked_amount.min(self.pool.locked_for_payouts);
            self.pool.available_liquidity += locked_amount;
            self.pool.theta_revenue += option.premium_paid;
        }
        
        option.status = OptionStatus::Settled;
        
        // Remove settled option from active options
        self.pool.active_options.remove(option_id);
        
        // Recalculate Greeks after removing option
        self.recalculate_pool_greeks();
        
        Ok(payout)
    }

    /// Recalculate pool Greeks from all active options
    fn recalculate_pool_greeks(&mut self) {
        self.pool.net_delta = 0.0;
        self.pool.net_gamma = 0.0;
        self.pool.net_vega = 0.0;
        self.pool.net_theta = 0.0;
        
        if let Some(price_data) = &self.price_cache {
            let spot = price_data.average_price as f64;
            
            for option in self.pool.active_options.values() {
                if option.status == OptionStatus::Active {
                    // Simplified Greeks calculation
                    let strike = option.strike_price as f64;
                    let time_to_expiry = (option.expiry_timestamp - chrono::Utc::now().timestamp() as u64) as f64 / 86400.0 / 365.0;
                    
                    // Delta calculation (simplified)
                    let moneyness = spot / strike;
                    let delta = match option.option_type {
                        OptionType::Call => 0.5 + 0.5 * moneyness.ln(),
                        OptionType::Put => -0.5 + 0.5 * moneyness.ln(),
                    }.max(-1.0).min(1.0);
                    
                    self.pool.net_delta += delta * (option.quantity as f64 / 1e8);
                    self.pool.net_theta += option.target_theta;
                }
            }
        }
    }
    
    /// Get pool statistics
    pub fn get_pool_stats(&self) -> &DeltaNeutralPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buy_call_option() {
        let mut manager = BuyerOnlyOptionManager::new(10_000_000); // 0.1 BTC
        
        // Set current price
        manager.update_price(AggregatedPrice {
            binance_price: 7000000,  // $70,000
            coinbase_price: 7005000, // $70,050
            kraken_price: 6995000,   // $69,950
            average_price: 7000000,  // $70,000
            timestamp: 1234567890,
        });
        
        // Buy a call option
        let result = manager.buy_option(
            OptionType::Call,
            7500000, // $75,000 strike
            1_000_000, // 0.01 BTC
            -0.02, // Target theta: -2% per day
            7.0, // 7 days to expiry
            "bc1qtest".to_string(),
        );
        
        assert!(result.is_ok());
        let option = result.unwrap();
        assert!(option.premium_paid > 0);
        assert_eq!(option.strike_price, 7500000);
        assert_eq!(manager.pool.active_options.len(), 1);
    }

    #[test]
    fn test_settle_itm_call() {
        let mut manager = BuyerOnlyOptionManager::new(10_000_000);
        
        manager.update_price(AggregatedPrice {
            binance_price: 7000000,
            coinbase_price: 7000000,
            kraken_price: 7000000,
            average_price: 7000000,
            timestamp: 1234567890,
        });
        
        let option = manager.buy_option(
            OptionType::Call,
            7000000, // ATM
            1_000_000,
            -0.02,
            1.0,
            "bc1qtest".to_string(),
        ).unwrap();
        
        // Settle at $75,000 (ITM)
        let payout = manager.settle_option(&option.option_id, 7500000).unwrap();
        assert!(payout > 0);
        
        // Check pool updated
        assert_eq!(manager.pool.total_payouts, payout);
    }
}
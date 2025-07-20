use anyhow::Result;
use btcfi_contracts::{SimplePoolState, OptionType};
use std::collections::HashMap;

/// 유동성 공급자
#[derive(Debug, Clone)]
pub struct LiquidityProvider {
    pub provider_id: String,
    pub deposited_amount: u64,  // satoshis
    pub shares: u64,            // LP tokens
}

/// 풀 매니저
pub struct PoolManager {
    pub state: SimplePoolState,
    pub providers: HashMap<String, LiquidityProvider>,
    pub total_shares: u64,
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            state: SimplePoolState::new(),
            providers: HashMap::new(),
            total_shares: 0,
        }
    }

    /// 유동성 추가
    pub fn add_liquidity(&mut self, provider_id: String, amount: u64) -> Result<u64> {
        if amount == 0 {
            anyhow::bail!("Amount must be greater than 0");
        }

        if amount < 100_000 { // 최소 0.001 BTC
            anyhow::bail!("Minimum deposit is 0.001 BTC");
        }

        // LP 토큰 계산
        let shares = if self.total_shares == 0 {
            // 첫 번째 공급자는 1:1 비율
            amount
        } else {
            // 기존 비율에 따라 계산
            (amount as u128 * self.total_shares as u128 / self.state.total_liquidity as u128) as u64
        };

        // 상태 업데이트
        self.state.total_liquidity += amount;
        self.state.available_liquidity += amount;
        self.total_shares += shares;

        // 공급자 정보 업데이트
        let provider = self.providers.entry(provider_id.clone()).or_insert(LiquidityProvider {
            provider_id,
            deposited_amount: 0,
            shares: 0,
        });
        provider.deposited_amount += amount;
        provider.shares += shares;

        Ok(shares)
    }

    /// 유동성 제거
    pub fn remove_liquidity(&mut self, provider_id: &str, shares: u64) -> Result<u64> {
        let provider = self.providers.get_mut(provider_id)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;

        if shares > provider.shares {
            anyhow::bail!("Insufficient shares");
        }

        if shares == 0 {
            anyhow::bail!("Shares must be greater than 0");
        }

        // 출금 금액 계산
        let withdraw_amount = (shares as u128 * self.state.total_liquidity as u128 / self.total_shares as u128) as u64;

        // 사용 가능한 유동성 확인
        if withdraw_amount > self.state.available_liquidity {
            anyhow::bail!("Insufficient available liquidity");
        }

        // 상태 업데이트
        self.state.total_liquidity -= withdraw_amount;
        self.state.available_liquidity -= withdraw_amount;
        self.total_shares -= shares;
        provider.shares -= shares;

        Ok(withdraw_amount)
    }

    /// 옵션을 위한 담보 잠금
    pub fn lock_collateral(&mut self, option_type: OptionType, quantity: u64, strike_price: u64) -> Result<()> {
        let required_collateral = match option_type {
            OptionType::Call => quantity, // Call은 수량만큼 필요
            OptionType::Put => {
                // Put은 행사가 기준 필요 (간단히 BTC=$70k 가정)
                (strike_price * quantity) / 7_000_000
            }
        };

        if required_collateral > self.state.available_liquidity {
            anyhow::bail!("Insufficient liquidity for collateral");
        }

        self.state.locked_collateral += required_collateral;
        self.state.available_liquidity -= required_collateral;
        self.state.active_options += 1;

        Ok(())
    }

    /// 담보 해제
    pub fn release_collateral(&mut self, option_type: OptionType, quantity: u64, strike_price: u64) -> Result<()> {
        let collateral_amount = match option_type {
            OptionType::Call => quantity,
            OptionType::Put => (strike_price * quantity) / 7_000_000,
        };

        if collateral_amount > self.state.locked_collateral {
            anyhow::bail!("Collateral amount exceeds locked amount");
        }

        self.state.locked_collateral -= collateral_amount;
        self.state.available_liquidity += collateral_amount;
        self.state.active_options = self.state.active_options.saturating_sub(1);

        Ok(())
    }

    /// 프리미엄 수령
    pub fn collect_premium(&mut self, premium: u64) -> Result<()> {
        if premium == 0 {
            anyhow::bail!("Premium must be greater than 0");
        }

        self.state.total_liquidity += premium;
        self.state.available_liquidity += premium;
        self.state.total_premium_collected += premium;

        Ok(())
    }

    /// 정산 지급
    pub fn payout_settlement(&mut self, amount: u64) -> Result<()> {
        if amount > self.state.locked_collateral {
            anyhow::bail!("Payout exceeds locked collateral");
        }

        self.state.locked_collateral -= amount;
        self.state.total_liquidity -= amount;
        self.state.total_payout += amount;

        Ok(())
    }

    /// 활용률 계산
    pub fn utilization_rate(&self) -> f64 {
        if self.state.total_liquidity == 0 {
            return 0.0;
        }

        (self.state.locked_collateral as f64 / self.state.total_liquidity as f64) * 100.0
    }

    /// LP 수익률 계산
    pub fn calculate_lp_return(&self, provider_id: &str) -> Option<f64> {
        let provider = self.providers.get(provider_id)?;
        
        if provider.shares == 0 {
            return Some(0.0);
        }

        let current_value = (provider.shares as f64 / self.total_shares as f64) * self.state.total_liquidity as f64;
        let initial_value = provider.deposited_amount as f64;

        Some(((current_value - initial_value) / initial_value) * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_first_liquidity() {
        // Given
        let mut pool = PoolManager::new();

        // When
        let shares = pool.add_liquidity("LP1".to_string(), 10_000_000).unwrap(); // 0.1 BTC

        // Then
        assert_eq!(shares, 10_000_000); // 1:1 for first provider
        assert_eq!(pool.state.total_liquidity, 10_000_000);
        assert_eq!(pool.state.available_liquidity, 10_000_000);
        assert_eq!(pool.total_shares, 10_000_000);
    }

    #[test]
    fn test_add_subsequent_liquidity() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 10_000_000).unwrap();
        
        // Pool now has premium income
        pool.collect_premium(1_000_000).unwrap(); // 0.01 BTC premium

        // When - Second LP adds same amount
        let shares = pool.add_liquidity("LP2".to_string(), 10_000_000).unwrap();

        // Then - Gets proportionally less shares due to premium
        assert!(shares < 10_000_000);
        assert_eq!(pool.state.total_liquidity, 21_000_000); // 0.1 + 0.01 + 0.1
    }

    #[test]
    fn test_remove_liquidity() {
        // Given
        let mut pool = PoolManager::new();
        let shares = pool.add_liquidity("LP1".to_string(), 10_000_000).unwrap();

        // When
        let withdrawn = pool.remove_liquidity("LP1", shares / 2).unwrap();

        // Then
        assert_eq!(withdrawn, 5_000_000);
        assert_eq!(pool.state.total_liquidity, 5_000_000);
        assert_eq!(pool.state.available_liquidity, 5_000_000);
    }

    #[test]
    fn test_reject_insufficient_liquidity() {
        // Given
        let mut pool = PoolManager::new();

        // When
        let result = pool.add_liquidity("LP1".to_string(), 50_000); // Less than minimum

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Minimum deposit is 0.001 BTC");
    }

    #[test]
    fn test_lock_call_collateral() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap(); // 1 BTC

        // When - Lock collateral for 0.5 BTC call option
        pool.lock_collateral(OptionType::Call, 50_000_000, 7_000_000).unwrap();

        // Then
        assert_eq!(pool.state.locked_collateral, 50_000_000);
        assert_eq!(pool.state.available_liquidity, 50_000_000);
        assert_eq!(pool.state.active_options, 1);
    }

    #[test]
    fn test_lock_put_collateral() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap();

        // When - Lock collateral for put option
        pool.lock_collateral(OptionType::Put, 10_000_000, 7_000_000).unwrap();

        // Then
        assert_eq!(pool.state.locked_collateral, 10_000_000); // Same as quantity at $70k
        assert_eq!(pool.state.available_liquidity, 90_000_000);
    }

    #[test]
    fn test_reject_insufficient_collateral() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 10_000_000).unwrap(); // 0.1 BTC

        // When - Try to lock more than available
        let result = pool.lock_collateral(OptionType::Call, 20_000_000, 7_000_000);

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Insufficient liquidity for collateral");
    }

    #[test]
    fn test_release_collateral() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap();
        pool.lock_collateral(OptionType::Call, 50_000_000, 7_000_000).unwrap();

        // When
        pool.release_collateral(OptionType::Call, 50_000_000, 7_000_000).unwrap();

        // Then
        assert_eq!(pool.state.locked_collateral, 0);
        assert_eq!(pool.state.available_liquidity, 100_000_000);
        assert_eq!(pool.state.active_options, 0);
    }

    #[test]
    fn test_collect_premium() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap();

        // When
        pool.collect_premium(1_000_000).unwrap(); // 0.01 BTC premium

        // Then
        assert_eq!(pool.state.total_premium_collected, 1_000_000);
        assert_eq!(pool.state.total_liquidity, 101_000_000);
        assert_eq!(pool.state.available_liquidity, 101_000_000);
    }

    #[test]
    fn test_payout_settlement() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap();
        pool.lock_collateral(OptionType::Call, 50_000_000, 7_000_000).unwrap();

        // When - Payout ITM option
        pool.payout_settlement(30_000_000).unwrap(); // 0.3 BTC payout

        // Then
        assert_eq!(pool.state.locked_collateral, 20_000_000);
        assert_eq!(pool.state.total_liquidity, 70_000_000);
        assert_eq!(pool.state.total_payout, 30_000_000);
    }

    #[test]
    fn test_utilization_rate() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap();
        pool.lock_collateral(OptionType::Call, 30_000_000, 7_000_000).unwrap();

        // When
        let utilization = pool.utilization_rate();

        // Then
        assert_eq!(utilization, 30.0); // 30%
    }

    #[test]
    fn test_lp_return_with_profit() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap();
        
        // Collect premiums
        pool.collect_premium(5_000_000).unwrap(); // 5% return

        // When
        let return_rate = pool.calculate_lp_return("LP1").unwrap();

        // Then
        assert_eq!(return_rate, 5.0); // 5% return
    }

    #[test]
    fn test_lp_return_with_loss() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap();
        pool.lock_collateral(OptionType::Call, 50_000_000, 7_000_000).unwrap();
        
        // Payout exceeds premium
        pool.payout_settlement(10_000_000).unwrap(); // 10% loss

        // When
        let return_rate = pool.calculate_lp_return("LP1").unwrap();

        // Then
        assert_eq!(return_rate, -10.0); // -10% return
    }

    #[test]
    fn test_multiple_providers_share_profits() {
        // Given
        let mut pool = PoolManager::new();
        pool.add_liquidity("LP1".to_string(), 60_000_000).unwrap(); // 60%
        pool.add_liquidity("LP2".to_string(), 40_000_000).unwrap(); // 40%
        
        // Collect premium
        pool.collect_premium(10_000_000).unwrap(); // 10% of initial

        // When
        let return_lp1 = pool.calculate_lp_return("LP1").unwrap();
        let return_lp2 = pool.calculate_lp_return("LP2").unwrap();

        // Then - Both get same return rate
        assert_eq!(return_lp1, 10.0);
        assert_eq!(return_lp2, 10.0);
    }

    #[test]
    fn test_prevent_withdrawal_with_locked_collateral() {
        // Given
        let mut pool = PoolManager::new();
        let shares = pool.add_liquidity("LP1".to_string(), 100_000_000).unwrap();
        pool.lock_collateral(OptionType::Call, 80_000_000, 7_000_000).unwrap();

        // When - Try to withdraw all
        let result = pool.remove_liquidity("LP1", shares);

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Insufficient available liquidity");
    }
}
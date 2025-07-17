use anyhow::Result;
use bitcoin::{Address, Amount, OutPoint, PublicKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 유동성 공급자 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityProvider {
    pub pubkey: PublicKey,
    pub deposited_amount: Amount,
    pub shares: u64,      // LP 토큰/지분
    pub last_update: u64, // 타임스탬프
    pub pending_withdrawal: Option<Amount>,
}

/// 풀 상태
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub total_liquidity: Amount,
    pub available_liquidity: Amount,
    pub locked_collateral: Amount,
    pub total_shares: u64,
    pub total_call_delta: f64,
    pub total_put_delta: f64,
    pub net_delta: f64,
    pub total_premium_collected: Amount,
    pub total_payout: Amount,
    pub last_update_height: u32,
}

impl PoolState {
    pub fn new() -> Self {
        Self {
            total_liquidity: Amount::ZERO,
            available_liquidity: Amount::ZERO,
            locked_collateral: Amount::ZERO,
            total_shares: 0,
            total_call_delta: 0.0,
            total_put_delta: 0.0,
            net_delta: 0.0,
            total_premium_collected: Amount::ZERO,
            total_payout: Amount::ZERO,
            last_update_height: 0,
        }
    }

    /// 활용률 계산 (%)
    pub fn utilization_rate(&self) -> f64 {
        if self.total_liquidity == Amount::ZERO {
            return 0.0;
        }

        let locked = self.locked_collateral.to_sat() as f64;
        let total = self.total_liquidity.to_sat() as f64;
        (locked / total) * 100.0
    }

    /// 델타 업데이트
    pub fn update_delta(&mut self, call_delta: f64, put_delta: f64) {
        self.total_call_delta = call_delta;
        self.total_put_delta = put_delta;
        self.net_delta = call_delta + put_delta;
    }
}

/// 풀 거래 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolTransaction {
    Deposit {
        provider: PublicKey,
        amount: Amount,
        shares_issued: u64,
    },
    Withdrawal {
        provider: PublicKey,
        amount: Amount,
        shares_burned: u64,
    },
    PremiumCollected {
        option_id: String,
        amount: Amount,
    },
    SettlementPayout {
        option_id: String,
        amount: Amount,
        recipient: PublicKey,
    },
    CollateralLocked {
        option_id: String,
        amount: Amount,
    },
    CollateralReleased {
        option_id: String,
        amount: Amount,
    },
}

/// 유동성 풀 관리자
pub struct PoolManager {
    pub state: PoolState,
    pub providers: HashMap<PublicKey, LiquidityProvider>,
    pub pool_address: Address,
    pub pool_utxos: Vec<(OutPoint, Amount)>,
    pub transaction_history: Vec<(u32, PoolTransaction)>, // (block_height, transaction)
}

impl PoolManager {
    pub fn new(pool_address: Address) -> Self {
        Self {
            state: PoolState::new(),
            providers: HashMap::new(),
            pool_address,
            pool_utxos: Vec::new(),
            transaction_history: Vec::new(),
        }
    }

    /// 유동성 추가
    pub fn add_liquidity(
        &mut self,
        provider: PublicKey,
        amount: Amount,
        block_height: u32,
    ) -> Result<u64> {
        // LP 토큰 계산
        let shares = if self.state.total_shares == 0 {
            // 첫 공급자는 1:1 비율
            amount.to_sat()
        } else {
            // 기존 비율에 따라 계산
            let share_price =
                self.state.total_liquidity.to_sat() as f64 / self.state.total_shares as f64;
            (amount.to_sat() as f64 / share_price) as u64
        };

        // 상태 업데이트
        self.state.total_liquidity += amount;
        self.state.available_liquidity += amount;
        self.state.total_shares += shares;

        // LP 정보 업데이트
        let lp = self.providers.entry(provider).or_insert(LiquidityProvider {
            pubkey: provider,
            deposited_amount: Amount::ZERO,
            shares: 0,
            last_update: block_height as u64,
            pending_withdrawal: None,
        });

        lp.deposited_amount += amount;
        lp.shares += shares;
        lp.last_update = block_height as u64;

        // 거래 기록
        self.transaction_history.push((
            block_height,
            PoolTransaction::Deposit {
                provider,
                amount,
                shares_issued: shares,
            },
        ));

        Ok(shares)
    }

    /// 유동성 제거
    pub fn remove_liquidity(
        &mut self,
        provider: PublicKey,
        shares: u64,
        block_height: u32,
    ) -> Result<Amount> {
        let lp = self
            .providers
            .get_mut(&provider)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;

        if lp.shares < shares {
            return Err(anyhow::anyhow!("Insufficient shares"));
        }

        // 출금 금액 계산
        let share_value =
            self.state.total_liquidity.to_sat() as f64 / self.state.total_shares as f64;
        let withdraw_amount = Amount::from_sat((shares as f64 * share_value) as u64);

        // 사용 가능한 유동성 확인
        if self.state.available_liquidity < withdraw_amount {
            return Err(anyhow::anyhow!("Insufficient available liquidity"));
        }

        // 상태 업데이트
        self.state.total_liquidity -= withdraw_amount;
        self.state.available_liquidity -= withdraw_amount;
        self.state.total_shares -= shares;

        lp.shares -= shares;
        lp.last_update = block_height as u64;

        // 거래 기록
        self.transaction_history.push((
            block_height,
            PoolTransaction::Withdrawal {
                provider,
                amount: withdraw_amount,
                shares_burned: shares,
            },
        ));

        Ok(withdraw_amount)
    }

    /// 프리미엄 수령
    pub fn collect_premium(
        &mut self,
        option_id: String,
        amount: Amount,
        block_height: u32,
    ) -> Result<()> {
        self.state.available_liquidity += amount;
        self.state.total_liquidity += amount;
        self.state.total_premium_collected += amount;

        self.transaction_history.push((
            block_height,
            PoolTransaction::PremiumCollected { option_id, amount },
        ));

        Ok(())
    }

    /// 담보금 잠금
    pub fn lock_collateral(
        &mut self,
        option_id: String,
        amount: Amount,
        block_height: u32,
    ) -> Result<()> {
        if self.state.available_liquidity < amount {
            return Err(anyhow::anyhow!("Insufficient available liquidity"));
        }

        self.state.available_liquidity -= amount;
        self.state.locked_collateral += amount;

        self.transaction_history.push((
            block_height,
            PoolTransaction::CollateralLocked { option_id, amount },
        ));

        Ok(())
    }

    /// 담보금 해제
    pub fn release_collateral(
        &mut self,
        option_id: String,
        amount: Amount,
        block_height: u32,
    ) -> Result<()> {
        if self.state.locked_collateral < amount {
            return Err(anyhow::anyhow!("Insufficient locked collateral"));
        }

        self.state.locked_collateral -= amount;
        self.state.available_liquidity += amount;

        self.transaction_history.push((
            block_height,
            PoolTransaction::CollateralReleased { option_id, amount },
        ));

        Ok(())
    }

    /// 정산 지급
    pub fn payout_settlement(
        &mut self,
        option_id: String,
        amount: Amount,
        recipient: PublicKey,
        block_height: u32,
    ) -> Result<()> {
        if amount > self.state.locked_collateral {
            return Err(anyhow::anyhow!("Payout exceeds locked collateral"));
        }

        self.state.locked_collateral -= amount;
        self.state.total_liquidity -= amount;
        self.state.total_payout += amount;

        self.transaction_history.push((
            block_height,
            PoolTransaction::SettlementPayout {
                option_id,
                amount,
                recipient,
            },
        ));

        Ok(())
    }

    /// UTXO 업데이트
    pub fn update_utxos(&mut self, utxos: Vec<(OutPoint, Amount)>) {
        self.pool_utxos = utxos;
    }

    /// LP 수익률 계산
    pub fn calculate_lp_returns(&self, provider: &PublicKey) -> Option<f64> {
        let lp = self.providers.get(provider)?;

        if lp.shares == 0 {
            return Some(0.0);
        }

        let current_value = (lp.shares as f64 / self.state.total_shares as f64)
            * self.state.total_liquidity.to_sat() as f64;
        let initial_value = lp.deposited_amount.to_sat() as f64;

        Some(((current_value - initial_value) / initial_value) * 100.0)
    }

    /// 리스크 지표 계산
    pub fn calculate_risk_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();

        // 활용률
        metrics.insert(
            "utilization_rate".to_string(),
            self.state.utilization_rate(),
        );

        // 델타 노출
        metrics.insert("net_delta".to_string(), self.state.net_delta);
        metrics.insert(
            "delta_ratio".to_string(),
            self.state.net_delta.abs()
                / (self.state.total_liquidity.to_sat() as f64 / 100_000_000.0),
        );

        // 수익성
        let profit = self.state.total_premium_collected.to_sat() as i64
            - self.state.total_payout.to_sat() as i64;
        metrics.insert(
            "total_profit_btc".to_string(),
            profit as f64 / 100_000_000.0,
        );

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{
        secp256k1::{rand::thread_rng, Secp256k1},
        Network,
    };

    #[test]
    fn test_liquidity_operations() {
        let pool_address = Address::p2pkh(
            &PublicKey::from_slice(&[0x02; 33]).unwrap(),
            Network::Testnet,
        );
        let mut pool = PoolManager::new(pool_address);

        let secp = Secp256k1::new();
        let (_, pubkey) = secp.generate_keypair(&mut thread_rng());
        let provider = PublicKey::from_slice(&pubkey.serialize()).unwrap();

        // 유동성 추가
        let shares = pool
            .add_liquidity(provider, Amount::from_sat(1_000_000), 100)
            .unwrap();

        assert_eq!(shares, 1_000_000);
        assert_eq!(pool.state.total_liquidity, Amount::from_sat(1_000_000));
        assert_eq!(pool.state.available_liquidity, Amount::from_sat(1_000_000));

        // 유동성 제거
        let withdrawn = pool.remove_liquidity(provider, 500_000, 101).unwrap();

        assert_eq!(withdrawn, Amount::from_sat(500_000));
        assert_eq!(pool.state.total_liquidity, Amount::from_sat(500_000));
    }

    #[test]
    fn test_collateral_management() {
        let pool_address = Address::p2pkh(
            &PublicKey::from_slice(&[0x02; 33]).unwrap(),
            Network::Testnet,
        );
        let mut pool = PoolManager::new(pool_address);

        // 초기 유동성
        pool.state.total_liquidity = Amount::from_sat(10_000_000);
        pool.state.available_liquidity = Amount::from_sat(10_000_000);

        // 담보금 잠금
        pool.lock_collateral("OPTION-001".to_string(), Amount::from_sat(1_000_000), 100)
            .unwrap();

        assert_eq!(pool.state.available_liquidity, Amount::from_sat(9_000_000));
        assert_eq!(pool.state.locked_collateral, Amount::from_sat(1_000_000));

        // 활용률 확인
        assert_eq!(pool.state.utilization_rate(), 10.0);
    }
}

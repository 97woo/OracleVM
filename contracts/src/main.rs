use anyhow::Result;
use bitcoin::secp256k1::{rand::thread_rng, Secp256k1};
use bitcoin::{Address, Amount, Network, PublicKey};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

mod bitcoin_utils;
mod option_contract;
mod pool_manager;
mod settlement;

use bitcoin_utils::TaprootAddressBuilder;
use option_contract::{
    OptionContract, OptionContractManager, OptionParams, OptionStatus, OptionType,
};
use pool_manager::PoolManager;
use settlement::SettlementEngine;

/// BTCFi 컨트랙트 시스템 메인 컨트롤러
pub struct BTCFiContractSystem {
    contract_manager: OptionContractManager,
    pool_manager: PoolManager,
    settlement_engine: SettlementEngine,
    taproot_builder: TaprootAddressBuilder,
    network: Network,
}

impl BTCFiContractSystem {
    pub fn new(network: Network) -> Self {
        // 풀 주소 생성 (테스트용)
        let secp = Secp256k1::new();
        let (_, pool_key) = secp.generate_keypair(&mut thread_rng());
        let pool_pubkey = PublicKey::from_slice(&pool_key.serialize()).unwrap();
        let pool_address = Address::p2pkh(&pool_pubkey, network);

        let pool_manager = PoolManager::new(pool_address);
        let settlement_engine = SettlementEngine::new(pool_manager.clone(), network);

        Self {
            contract_manager: OptionContractManager::new(),
            pool_manager,
            settlement_engine,
            taproot_builder: TaprootAddressBuilder::new(network),
            network,
        }
    }

    /// 새 옵션 컨트랙트 생성
    pub async fn create_option_contract(
        &mut self,
        user_pubkey: PublicKey,
        params: OptionParams,
    ) -> Result<String> {
        let contract_id = format!(
            "OPT-{}-{}-{}",
            match params.option_type {
                OptionType::Call => "CALL",
                OptionType::Put => "PUT",
            },
            params.strike_price / 100_000_000, // BTC 단위로 변환
            params.expiry_height
        );

        // Pool 공개키 (실제로는 멀티시그)
        let secp = Secp256k1::new();
        let (_, pool_key) = secp.generate_keypair(&mut thread_rng());
        let pool_pubkey = PublicKey::from_slice(&pool_key.serialize()).unwrap();

        // BitVMX commitment 생성
        let bitvmx_commitment = self.generate_bitvmx_commitment(&params)?;

        // Taproot 컨트랙트 주소 생성
        let (contract_address, _) = self.taproot_builder.create_option_contract_address(
            user_pubkey,
            pool_pubkey,
            bitvmx_commitment,
            params.expiry_height,
        )?;

        // 컨트랙트 생성
        let contract = OptionContract::new(
            contract_id.clone(),
            params.clone(),
            user_pubkey,
            contract_address,
            bitvmx_commitment,
        );

        // 담보금 잠금
        let current_height = 800_000; // 실제로는 현재 블록 높이
        self.pool_manager.lock_collateral(
            contract_id.clone(),
            contract.collateral_amount,
            current_height,
        )?;

        // 프리미엄 수령
        self.pool_manager
            .collect_premium(contract_id.clone(), params.premium, current_height)?;

        // 컨트랙트 등록
        self.contract_manager.add_contract(contract)?;

        info!("Created option contract: {}", contract_id);
        info!("Contract address: {}", contract_address);
        info!("Premium: {} sats", params.premium.to_sat());

        Ok(contract_id)
    }

    /// 유동성 추가
    pub async fn add_liquidity(&mut self, provider: PublicKey, amount: Amount) -> Result<u64> {
        let current_height = 800_000; // 실제로는 현재 블록 높이
        let shares = self
            .pool_manager
            .add_liquidity(provider, amount, current_height)?;

        info!(
            "Added liquidity: {} sats from {:?}, received {} shares",
            amount.to_sat(),
            provider,
            shares
        );

        Ok(shares)
    }

    /// 만료된 옵션 자동 정산
    pub async fn process_expired_options(&mut self, spot_price: u64) -> Result<Vec<String>> {
        let current_height = 800_001; // 만료 후 블록
        let expired_contracts = self.contract_manager.get_expired_contracts(current_height);

        let mut spot_prices = HashMap::new();
        spot_prices.insert("BTC".to_string(), spot_price);

        let processed = self
            .settlement_engine
            .process_expired_options(
                expired_contracts.into_iter().cloned().collect(),
                spot_prices,
                current_height,
            )
            .await?;

        // 컨트랙트 상태 업데이트
        for settlement_id in &processed {
            if let Some(contract_id) = settlement_id.split('-').nth(1) {
                let _ = self
                    .contract_manager
                    .update_status(contract_id, OptionStatus::Settled);
            }
        }

        info!("Processed {} expired options", processed.len());
        Ok(processed)
    }

    /// 시스템 상태 조회
    pub fn get_system_status(&self) -> HashMap<String, serde_json::Value> {
        let mut status = HashMap::new();

        status.insert(
            "pool_state".to_string(),
            serde_json::to_value(&self.pool_manager.state).unwrap(),
        );

        let risk_metrics = self.pool_manager.calculate_risk_metrics();
        status.insert(
            "risk_metrics".to_string(),
            serde_json::to_value(risk_metrics).unwrap(),
        );

        status
    }

    /// BitVMX commitment 생성 (시뮬레이션)
    fn generate_bitvmx_commitment(&self, params: &OptionParams) -> Result<[u8; 32]> {
        // 실제로는 BitVMX 모듈과 연동하여 commitment 생성
        use bitcoin::hashes::{sha256, Hash};

        let data = format!(
            "{:?}-{}-{}",
            params.option_type, params.strike_price, params.expiry_height
        );

        let hash = sha256::Hash::hash(data.as_bytes());
        Ok(hash.to_byte_array())
    }

    /// 메인 실행 루프
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting BTCFi Contract System...");
        info!("Network: {:?}", self.network);
        info!("Pool address: {}", self.pool_manager.pool_address);

        // 테스트용 시나리오 실행
        self.run_test_scenario().await?;

        // 실제 운영에서는 이벤트 기반 처리
        loop {
            // 1. 만료된 옵션 체크 및 처리
            let spot_price = 72_000_000_000_000u64; // $72,000
            match self.process_expired_options(spot_price).await {
                Ok(processed) => {
                    if !processed.is_empty() {
                        info!("Auto-processed {} expired options", processed.len());
                    }
                }
                Err(e) => error!("Error processing expired options: {}", e),
            }

            // 2. 시스템 상태 출력
            let status = self.get_system_status();
            info!("System status: {}", serde_json::to_string_pretty(&status)?);

            sleep(Duration::from_secs(60)).await;
        }
    }

    /// 테스트 시나리오 실행
    async fn run_test_scenario(&mut self) -> Result<()> {
        info!("=== Running Test Scenario ===");

        let secp = Secp256k1::new();

        // 1. LP 추가
        let (_, lp_key) = secp.generate_keypair(&mut thread_rng());
        let lp_pubkey = PublicKey::from_slice(&lp_key.serialize()).unwrap();

        self.add_liquidity(lp_pubkey, Amount::from_sat(10_000_000))
            .await?;

        // 2. 사용자 및 옵션 생성
        let (_, user_key) = secp.generate_keypair(&mut thread_rng());
        let user_pubkey = PublicKey::from_slice(&user_key.serialize()).unwrap();

        // Call 옵션 생성
        let call_params = OptionParams {
            option_type: OptionType::Call,
            strike_price: 70_000_000_000_000, // $70,000
            quantity: 10_000_000,             // 0.1 BTC
            expiry_height: 800_000,
            premium: Amount::from_sat(250_000), // 0.0025 BTC
        };

        let call_contract_id = self
            .create_option_contract(user_pubkey, call_params)
            .await?;

        // Put 옵션 생성
        let put_params = OptionParams {
            option_type: OptionType::Put,
            strike_price: 65_000_000_000_000, // $65,000
            quantity: 20_000_000,             // 0.2 BTC
            expiry_height: 800_000,
            premium: Amount::from_sat(180_000), // 0.0018 BTC
        };

        let put_contract_id = self.create_option_contract(user_pubkey, put_params).await?;

        info!(
            "Created test contracts: {} and {}",
            call_contract_id, put_contract_id
        );
        info!("=== Test Scenario Complete ===");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let network = Network::Testnet;
    let mut system = BTCFiContractSystem::new(network);

    info!("BTCFi Contract System starting...");
    system.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_contract_creation() {
        let mut system = BTCFiContractSystem::new(Network::Testnet);

        let secp = Secp256k1::new();
        let (_, user_key) = secp.generate_keypair(&mut thread_rng());
        let user_pubkey = PublicKey::from_slice(&user_key.serialize()).unwrap();

        let params = OptionParams {
            option_type: OptionType::Call,
            strike_price: 70_000_000_000_000,
            quantity: 10_000_000,
            expiry_height: 800_000,
            premium: Amount::from_sat(250_000),
        };

        // 먼저 유동성 추가
        let (_, lp_key) = secp.generate_keypair(&mut thread_rng());
        let lp_pubkey = PublicKey::from_slice(&lp_key.serialize()).unwrap();
        system
            .add_liquidity(lp_pubkey, Amount::from_sat(10_000_000))
            .await
            .unwrap();

        let contract_id = system
            .create_option_contract(user_pubkey, params)
            .await
            .unwrap();
        assert!(contract_id.starts_with("OPT-CALL-70000"));
    }
}

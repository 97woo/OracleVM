use anyhow::Result;
use bitcoin::{Address, Amount, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::bitcoin_utils::TransactionBuilder;
use crate::option_contract::{OptionContract, OptionStatus};
use crate::pool_manager::PoolManager;

/// BitVMX 정산 증명
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementProof {
    pub option_id: String,
    pub spot_price: u64,
    pub is_itm: bool,
    pub settlement_amount: Amount,
    pub bitvmx_proof: Vec<u8>,
    pub merkle_root: [u8; 32],
    pub block_height: u32,
}

/// 정산 상태
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementStatus {
    Pending,
    ProofSubmitted,
    Validated,
    Executed,
    Failed(String),
}

/// 정산 요청
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRequest {
    pub request_id: String,
    pub option_contract: OptionContract,
    pub spot_price: u64,
    pub proof: Option<SettlementProof>,
    pub status: SettlementStatus,
    pub created_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_tx: Option<String>, // Txid를 String으로 직렬화
}

/// 정산 엔진
pub struct SettlementEngine {
    pending_settlements: HashMap<String, SettlementRequest>,
    executed_settlements: HashMap<String, SettlementRequest>,
    pool_manager: PoolManager,
    tx_builder: TransactionBuilder,
}

impl SettlementEngine {
    pub fn new(pool_manager: PoolManager, network: bitcoin::Network) -> Self {
        Self {
            pending_settlements: HashMap::new(),
            executed_settlements: HashMap::new(),
            pool_manager,
            tx_builder: TransactionBuilder::new(network),
        }
    }

    /// 정산 요청 생성
    pub fn create_settlement_request(
        &mut self,
        contract: OptionContract,
        spot_price: u64,
    ) -> Result<String> {
        let request_id = format!(
            "SETTLE-{}-{}",
            contract.contract_id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
        );

        let request = SettlementRequest {
            request_id: request_id.clone(),
            option_contract: contract,
            spot_price,
            proof: None,
            status: SettlementStatus::Pending,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            settlement_tx: None,
        };

        self.pending_settlements.insert(request_id.clone(), request);

        Ok(request_id)
    }

    /// BitVMX 증명 제출
    pub fn submit_proof(&mut self, request_id: &str, proof: SettlementProof) -> Result<()> {
        let request = self
            .pending_settlements
            .get_mut(request_id)
            .ok_or_else(|| anyhow::anyhow!("Settlement request not found"))?;

        // 증명 검증
        self.validate_proof(&request.option_contract, &proof)?;

        request.proof = Some(proof);
        request.status = SettlementStatus::ProofSubmitted;

        Ok(())
    }

    /// 증명 검증
    fn validate_proof(&self, contract: &OptionContract, proof: &SettlementProof) -> Result<()> {
        // 옵션 ID 일치 확인
        if proof.option_id != contract.contract_id {
            return Err(anyhow::anyhow!("Option ID mismatch"));
        }

        // BitVMX commitment 확인
        // 실제로는 merkle proof 검증 등이 필요
        if proof.merkle_root != contract.bitvmx_commitment {
            return Err(anyhow::anyhow!("Invalid BitVMX commitment"));
        }

        // 정산 금액 검증
        let expected_amount = contract.calculate_settlement(proof.spot_price);
        if proof.settlement_amount != expected_amount {
            return Err(anyhow::anyhow!("Settlement amount mismatch"));
        }

        Ok(())
    }

    /// 정산 실행
    pub async fn execute_settlement(
        &mut self,
        request_id: &str,
        user_address: Address,
        pool_address: Address,
    ) -> Result<Transaction> {
        let mut request = self
            .pending_settlements
            .remove(request_id)
            .ok_or_else(|| anyhow::anyhow!("Settlement request not found"))?;

        if request.status != SettlementStatus::ProofSubmitted {
            return Err(anyhow::anyhow!("Proof not submitted"));
        }

        let proof = request
            .proof
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Proof not found"))?;

        let contract = &request.option_contract;
        let option_utxo = contract
            .get_utxo()
            .ok_or_else(|| anyhow::anyhow!("Contract UTXO not found"))?;

        // 정산 트랜잭션 생성
        let settlement_tx = self.tx_builder.build_settlement_tx(
            option_utxo,
            contract.collateral_amount,
            proof.settlement_amount,
            user_address,
            pool_address,
            proof.bitvmx_proof.clone(),
            Amount::from_sat(1000), // 수수료
        )?;

        // 풀 상태 업데이트
        if proof.is_itm {
            self.pool_manager.payout_settlement(
                contract.contract_id.clone(),
                proof.settlement_amount,
                contract.user_pubkey,
                proof.block_height,
            )?;
        } else {
            // OTM인 경우 담보금 풀로 반환
            self.pool_manager.release_collateral(
                contract.contract_id.clone(),
                contract.collateral_amount,
                proof.block_height,
            )?;
        }

        request.status = SettlementStatus::Executed;
        request.settlement_tx = Some(settlement_tx.compute_txid().to_string());

        self.executed_settlements
            .insert(request_id.to_string(), request);

        Ok(settlement_tx)
    }

    /// 만료된 옵션 자동 정산 처리
    pub async fn process_expired_options(
        &mut self,
        contracts: Vec<OptionContract>,
        spot_prices: HashMap<String, u64>,
        block_height: u32,
    ) -> Result<Vec<String>> {
        let mut processed = Vec::new();

        for contract in contracts {
            if contract.status != OptionStatus::Active {
                continue;
            }

            // 적절한 spot price 찾기
            let spot_price = spot_prices
                .get(&contract.contract_id)
                .or_else(|| spot_prices.get("BTC"))
                .copied()
                .ok_or_else(|| anyhow::anyhow!("No spot price available"))?;

            // 정산 요청 생성
            let request_id = self.create_settlement_request(contract.clone(), spot_price)?;

            // BitVMX 증명 생성 (실제로는 BitVMX 모듈 호출)
            let proof = self.generate_bitvmx_proof(&contract, spot_price, block_height)?;

            // 증명 제출
            self.submit_proof(&request_id, proof)?;

            processed.push(request_id);
        }

        Ok(processed)
    }

    /// BitVMX 증명 생성 (시뮬레이션)
    fn generate_bitvmx_proof(
        &self,
        contract: &OptionContract,
        spot_price: u64,
        block_height: u32,
    ) -> Result<SettlementProof> {
        let is_itm = contract.is_in_the_money(spot_price);
        let settlement_amount = contract.calculate_settlement(spot_price);

        // 실제로는 BitVMX 모듈을 호출하여 증명 생성
        let proof = SettlementProof {
            option_id: contract.contract_id.clone(),
            spot_price,
            is_itm,
            settlement_amount,
            bitvmx_proof: vec![0u8; 64], // 실제 증명 데이터
            merkle_root: contract.bitvmx_commitment,
            block_height,
        };

        Ok(proof)
    }

    /// 정산 상태 조회
    pub fn get_settlement_status(&self, request_id: &str) -> Option<SettlementStatus> {
        self.pending_settlements
            .get(request_id)
            .or_else(|| self.executed_settlements.get(request_id))
            .map(|req| req.status.clone())
    }

    /// 정산 이력 조회
    pub fn get_settlement_history(&self, option_id: &str) -> Vec<&SettlementRequest> {
        self.executed_settlements
            .values()
            .filter(|req| req.option_contract.contract_id == option_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::option_contract::{OptionParams, OptionType};
    use bitcoin::{
        secp256k1::{rand::thread_rng, Secp256k1},
        Network,
    };

    #[test]
    fn test_settlement_request_creation() {
        let pool_address = Address::p2pkh(
            &PublicKey::from_slice(&[0x02; 33]).unwrap(),
            Network::Testnet,
        );
        let pool_manager = PoolManager::new(pool_address);
        let mut engine = SettlementEngine::new(pool_manager, Network::Testnet);

        let secp = Secp256k1::new();
        let (_, pubkey) = secp.generate_keypair(&mut thread_rng());
        let user_pubkey = PublicKey::from_slice(&pubkey.serialize()).unwrap();

        let params = OptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000_000_000,
            quantity: 10_000_000,
            expiry_height: 800_000,
            premium: Amount::from_sat(250_000),
        };

        let contract = OptionContract::new(
            "TEST-001".to_string(),
            params,
            user_pubkey,
            Address::p2pkh(&user_pubkey, Network::Testnet),
            [0u8; 32],
        );

        let request_id = engine
            .create_settlement_request(
                contract,
                7_200_000_000_000, // spot price
            )
            .unwrap();

        assert!(request_id.starts_with("SETTLE-TEST-001"));
        assert!(engine.pending_settlements.contains_key(&request_id));
    }

    #[test]
    fn test_proof_validation() {
        let pool_address = Address::p2pkh(
            &PublicKey::from_slice(&[0x02; 33]).unwrap(),
            Network::Testnet,
        );
        let pool_manager = PoolManager::new(pool_address);
        let engine = SettlementEngine::new(pool_manager, Network::Testnet);

        let secp = Secp256k1::new();
        let (_, pubkey) = secp.generate_keypair(&mut thread_rng());
        let user_pubkey = PublicKey::from_slice(&pubkey.serialize()).unwrap();

        let params = OptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000_000_000,
            quantity: 10_000_000,
            expiry_height: 800_000,
            premium: Amount::from_sat(250_000),
        };

        let contract = OptionContract::new(
            "TEST-001".to_string(),
            params,
            user_pubkey,
            Address::p2pkh(&user_pubkey, Network::Testnet),
            [1u8; 32],
        );

        let spot_price = 7_200_000_000_000u64;
        let settlement_amount = contract.calculate_settlement(spot_price);

        let proof = SettlementProof {
            option_id: "TEST-001".to_string(),
            spot_price,
            is_itm: true,
            settlement_amount,
            bitvmx_proof: vec![0u8; 64],
            merkle_root: [1u8; 32],
            block_height: 800_001,
        };

        assert!(engine.validate_proof(&contract, &proof).is_ok());
    }
}

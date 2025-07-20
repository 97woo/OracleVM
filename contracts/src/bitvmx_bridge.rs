use crate::bitcoin_option::{BitcoinOption, OptionType};
use anyhow::Result;
use bitcoin::hashes::{sha256, Hash};
use std::process::Command;

/// BitVMX와 Bitcoin 옵션을 연결하는 브릿지
/// 오프체인에서 가격을 받아 BitVMX로 증명을 생성하고
/// 온체인에서 검증 가능한 형태로 변환
pub struct BitVmxBridge {
    /// BitVMX 바이너리 경로
    bitvmx_path: String,
    /// 옵션 정산 프로그램 경로
    settlement_program: String,
}

impl BitVmxBridge {
    pub fn new() -> Self {
        Self {
            bitvmx_path: "../bitvmx_protocol/BitVMX-CPU/target/release/emulator".to_string(),
            settlement_program: "../bitvmx_protocol/execution_files/option_settlement.elf".to_string(),
        }
    }
    
    /// Oracle 가격 데이터를 BitVMX 입력 형식으로 변환
    pub fn prepare_settlement_input(
        &self,
        option: &BitcoinOption,
        spot_price: u64,
    ) -> Vec<u8> {
        let mut input = Vec::with_capacity(16);
        
        // Option type (4 bytes)
        let option_type_bytes = match option.option_type {
            OptionType::Call => 0u32,
            OptionType::Put => 1u32,
        };
        input.extend_from_slice(&option_type_bytes.to_le_bytes());
        
        // Strike price in cents (4 bytes)
        let strike_cents = (option.strike_price / 1_000) as u32; // satoshis to cents
        input.extend_from_slice(&strike_cents.to_le_bytes());
        
        // Spot price in cents (4 bytes)
        let spot_cents = (spot_price / 1_000) as u32;
        input.extend_from_slice(&spot_cents.to_le_bytes());
        
        // Quantity (4 bytes) - simplified to 1 unit
        let quantity = 100u32; // 1.00 in fixed point
        input.extend_from_slice(&quantity.to_le_bytes());
        
        input
    }
    
    /// BitVMX를 실행하여 정산 증명 생성
    pub async fn generate_settlement_proof(
        &self,
        option: &BitcoinOption,
        spot_price: u64,
    ) -> Result<SettlementProof> {
        let input = self.prepare_settlement_input(option, spot_price);
        let input_hex = hex::encode(&input);
        
        // BitVMX 에뮬레이터 실행
        let output = Command::new(&self.bitvmx_path)
            .arg("execute")
            .arg("--elf")
            .arg(&self.settlement_program)
            .arg("--input")
            .arg(&input_hex)
            .arg("--trace")
            .output()?;
            
        if !output.status.success() {
            anyhow::bail!("BitVMX execution failed");
        }
        
        // 실행 결과 파싱
        let stdout = String::from_utf8(output.stdout)?;
        let settlement_amount = self.parse_settlement_amount(&stdout)?;
        
        // 증명 데이터 구성
        let proof_data = self.create_proof_data(
            option,
            spot_price,
            settlement_amount,
        );
        
        // 증명 해시 계산
        let proof_hash = sha256::Hash::hash(&proof_data);
        
        Ok(SettlementProof {
            proof_data,
            proof_hash: proof_hash.to_byte_array(),
            settlement_amount,
            execution_trace: stdout,
        })
    }
    
    /// BitVMX 출력에서 정산 금액 파싱
    fn parse_settlement_amount(&self, output: &str) -> Result<u64> {
        // BitVMX 출력 형식: "Settlement amount: XXXX cents"
        for line in output.lines() {
            if line.contains("Settlement amount:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let cents: u64 = parts[2].parse()?;
                    // cents to satoshis (assuming 1 BTC = $100,000)
                    return Ok(cents * 1_000);
                }
            }
        }
        
        // ITM이 아니면 0 반환
        Ok(0)
    }
    
    /// 온체인 검증을 위한 증명 데이터 생성
    fn create_proof_data(
        &self,
        option: &BitcoinOption,
        spot_price: u64,
        settlement_amount: u64,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        
        // 옵션 파라미터
        data.push(match option.option_type {
            OptionType::Call => 0,
            OptionType::Put => 1,
        });
        data.extend_from_slice(&option.strike_price.to_le_bytes());
        data.extend_from_slice(&spot_price.to_le_bytes());
        data.extend_from_slice(&settlement_amount.to_le_bytes());
        
        // 타임스탬프 추가
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        data.extend_from_slice(&timestamp.to_le_bytes());
        
        data
    }
    
    /// 증명 검증 (온체인 스크립트 시뮬레이션)
    pub fn verify_proof(
        &self,
        proof: &SettlementProof,
        expected_hash: &[u8; 32],
    ) -> bool {
        let computed_hash = sha256::Hash::hash(&proof.proof_data);
        &computed_hash.to_byte_array() == expected_hash
    }
}

/// 정산 증명 구조체
#[derive(Debug, Clone)]
pub struct SettlementProof {
    /// 증명 데이터
    pub proof_data: Vec<u8>,
    /// 증명 해시 (온체인 검증용)
    pub proof_hash: [u8; 32],
    /// 정산 금액 (satoshis)
    pub settlement_amount: u64,
    /// BitVMX 실행 트레이스
    pub execution_trace: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey};
    use bitcoin::secp256k1::rand::thread_rng;
    
    #[test]
    fn test_prepare_settlement_input() {
        let bridge = BitVmxBridge::new();
        let secp = Secp256k1::new();
        let mut rng = thread_rng();
        
        let option = BitcoinOption {
            option_type: OptionType::Call,
            strike_price: 50_000_000_000, // $50k in satoshis
            expiry_block: 800_000,
            buyer_pubkey: PublicKey::from_secret_key(&secp, &SecretKey::new(&mut rng)),
            seller_pubkey: PublicKey::from_secret_key(&secp, &SecretKey::new(&mut rng)),
            verifier_pubkey: PublicKey::from_secret_key(&secp, &SecretKey::new(&mut rng)),
            premium: 1_000_000_000,
            collateral: 10_000_000_000,
        };
        
        let input = bridge.prepare_settlement_input(&option, 52_000_000_000);
        
        // Verify input format
        assert_eq!(input.len(), 16);
        
        // Check option type
        assert_eq!(&input[0..4], &[0, 0, 0, 0]); // Call = 0
        
        // Check strike price (50M satoshis = 50M/1000 = 50k cents)
        let strike_bytes = &input[4..8];
        let strike = u32::from_le_bytes(strike_bytes.try_into().unwrap());
        assert_eq!(strike, 50_000_000);
    }
    
    #[test]
    fn test_proof_verification() {
        let bridge = BitVmxBridge::new();
        
        let proof_data = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let proof_hash = sha256::Hash::hash(&proof_data).to_byte_array();
        
        let proof = SettlementProof {
            proof_data: proof_data.clone(),
            proof_hash,
            settlement_amount: 1_000_000,
            execution_trace: "test trace".to_string(),
        };
        
        // Should verify with correct hash
        assert!(bridge.verify_proof(&proof, &proof_hash));
        
        // Should fail with wrong hash
        let wrong_hash = [0u8; 32];
        assert!(!bridge.verify_proof(&proof, &wrong_hash));
    }
}
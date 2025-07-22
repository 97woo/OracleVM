//! BitVMX Pre-sign Transaction 생성 (간소화 버전)

use anyhow::Result;
use bitcoin::{
    Transaction, TxOut, TxIn, OutPoint, Witness,
    ScriptBuf, Address, Network,
    secp256k1::{Secp256k1, SecretKey, PublicKey},
    Amount, locktime::absolute::LockTime, Sequence,
};
use crate::bitvmx_proof_generator::SettlementResult;

/// Pre-signed 옵션 정산 트랜잭션 생성기
pub struct PreSignedSettlementBuilder {
    secp: Secp256k1<bitcoin::secp256k1::All>,
    network: Network,
}

impl PreSignedSettlementBuilder {
    /// 새로운 빌더 생성
    pub fn new(network: Network) -> Self {
        Self {
            secp: Secp256k1::new(),
            network,
        }
    }
    
    /// 옵션 정산을 위한 pre-signed transaction 생성
    pub fn create_settlement_transaction(
        &self,
        option_utxo: OutPoint,
        option_value: Amount,
        buyer_key: &SecretKey,
        _operator_key: &SecretKey,
        settlement_script: ScriptBuf,
        expiry_height: u32,
    ) -> Result<(Transaction, Vec<Vec<u8>>)> {
        // 매수자 주소 생성
        let buyer_pubkey = PublicKey::from_secret_key(&self.secp, buyer_key);
        let compressed_pubkey = bitcoin::key::CompressedPublicKey::from_private_key(
            &self.secp,
            &bitcoin::key::PrivateKey::new(*buyer_key, self.network)
        ).unwrap();
        let buyer_address = Address::p2wpkh(&compressed_pubkey, self.network);
        
        // 정산 트랜잭션 생성
        let tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: LockTime::from_height(expiry_height).unwrap(),
            input: vec![TxIn {
                previous_output: option_utxo,
                script_sig: ScriptBuf::new(),
                sequence: Sequence(0xfffffffd), // RBF 활성화
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: option_value - Amount::from_sat(1000), // 수수료 제외
                script_pubkey: buyer_address.script_pubkey(),
            }],
        };
        
        // 간소화된 witness 템플릿
        let witness_template = vec![
            vec![], // 서명 플레이스홀더
            vec![], // 증명 플레이스홀더
            settlement_script.to_bytes(),
        ];
        
        Ok((tx, witness_template))
    }
    
    /// 매수자가 증명을 추가하여 트랜잭션 완성
    pub fn complete_with_proof(
        &self,
        mut tx: Transaction,
        mut witness_template: Vec<Vec<u8>>,
        proof_scripts: Vec<ScriptBuf>,
        settlement_result: &SettlementResult,
    ) -> Result<Transaction> {
        // 증명 데이터 구성
        let mut proof_data = Vec::new();
        
        // 정산 결과
        proof_data.push(settlement_result.is_itm as u8);
        proof_data.extend_from_slice(&settlement_result.intrinsic_value.to_le_bytes());
        proof_data.extend_from_slice(&settlement_result.settlement_amount.to_le_bytes());
        
        // 증명 스크립트 직렬화
        for script in &proof_scripts {
            proof_data.extend_from_slice(&(script.len() as u16).to_le_bytes());
            proof_data.extend_from_slice(script.as_bytes());
        }
        
        // Witness에 증명 추가
        witness_template[1] = proof_data;
        
        // 더미 서명 추가 (실제로는 적절한 서명 필요)
        witness_template[0] = vec![0; 64];
        
        // 트랜잭션에 witness 설정
        tx.input[0].witness = Witness::from(witness_template);
        
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hashes::Hash;
    
    #[test]
    fn test_presigned_settlement() {
        let builder = PreSignedSettlementBuilder::new(Network::Testnet);
        
        // 테스트 키
        let buyer_key = SecretKey::from_slice(&[1u8; 32]).unwrap();
        let operator_key = SecretKey::from_slice(&[2u8; 32]).unwrap();
        
        // 테스트 UTXO
        let option_utxo = OutPoint {
            txid: bitcoin::Txid::all_zeros(),
            vout: 0,
        };
        
        // 간단한 정산 스크립트
        let settlement_script = ScriptBuf::from(vec![
            bitcoin::opcodes::all::OP_PUSHNUM_1.to_u8(),
        ]);
        
        // Pre-signed transaction 생성
        let (tx, witness) = builder.create_settlement_transaction(
            option_utxo,
            Amount::from_sat(100_000),
            &buyer_key,
            &operator_key,
            settlement_script,
            800_000, // 만기 블록
        ).unwrap();
        
        assert_eq!(tx.input.len(), 1);
        assert_eq!(tx.output.len(), 1);
        assert_eq!(witness.len(), 3);
    }
}
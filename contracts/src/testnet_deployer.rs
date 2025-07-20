use crate::bitcoin_option::BitcoinOption;
use oracle_vm_common::types::OptionType;
use bitcoin::{
    Network, Transaction, TxIn, TxOut, OutPoint, Sequence, Witness,
    Amount, Address, ScriptBuf, absolute::LockTime,
};
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::{CompressedPublicKey, PublicKey};
use anyhow::Result;

/// Bitcoin Testnet 배포 및 테스트 도구
pub struct TestnetDeployer {
    network: Network,
    secp: Secp256k1<bitcoin::secp256k1::All>,
}

impl TestnetDeployer {
    pub fn new() -> Self {
        Self {
            network: Network::Testnet,
            secp: Secp256k1::new(),
        }
    }
    
    /// 옵션 생성 트랜잭션 만들기
    /// 구매자가 프리미엄을 지불하고, 판매자가 담보를 잠그는 트랜잭션
    pub fn create_option_funding_tx(
        &self,
        option: &BitcoinOption,
        buyer_utxo: OutPoint,
        buyer_utxo_amount: Amount,
        seller_utxo: OutPoint,
        seller_utxo_amount: Amount,
        buyer_key: &SecretKey,
        seller_key: &SecretKey,
    ) -> Result<Transaction> {
        // Taproot 스크립트 생성
        let (taproot_script, spend_info) = option.create_taproot_script()?;
        
        // 입력 생성
        let buyer_input = TxIn {
            previous_output: buyer_utxo,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        };
        
        let seller_input = TxIn {
            previous_output: seller_utxo,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        };
        
        // 출력 생성
        // 1. 옵션 컨트랙트 출력 (프리미엄 + 담보)
        let option_output = TxOut {
            value: Amount::from_sat(option.premium + option.collateral),
            script_pubkey: taproot_script.clone(),
        };
        
        // 2. 구매자 잔액 반환 (수수료 제외)
        let buyer_change = buyer_utxo_amount - Amount::from_sat(option.premium) - Amount::from_sat(1000); // 1000 sats 수수료
        let buyer_change_output = TxOut {
            value: buyer_change,
            script_pubkey: {
                let secp_pubkey = bitcoin::secp256k1::PublicKey::from_secret_key(&self.secp, buyer_key);
                let pubkey = PublicKey::from_private_key(&self.secp, &bitcoin::PrivateKey::new(*buyer_key, self.network));
                let compressed = CompressedPublicKey::try_from(pubkey).unwrap();
                Address::p2wpkh(&compressed, self.network).script_pubkey()
            },
        };
        
        // 3. 판매자 잔액 반환 (수수료 제외)
        let seller_change = seller_utxo_amount - Amount::from_sat(option.collateral) - Amount::from_sat(1000);
        let seller_change_output = TxOut {
            value: seller_change,
            script_pubkey: {
                let secp_pubkey = bitcoin::secp256k1::PublicKey::from_secret_key(&self.secp, seller_key);
                let pubkey = PublicKey::from_private_key(&self.secp, &bitcoin::PrivateKey::new(*seller_key, self.network));
                let compressed = CompressedPublicKey::try_from(pubkey).unwrap();
                Address::p2wpkh(&compressed, self.network).script_pubkey()
            },
        };
        
        // 트랜잭션 조립
        let mut tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![buyer_input, seller_input],
            output: vec![option_output, buyer_change_output, seller_change_output],
        };
        
        // 서명 생성 (실제 구현에서는 각 입력에 대해 적절한 서명 필요)
        // 여기서는 예시로 단순화
        println!("⚠️  실제 배포시 서명 필요: 구매자와 판매자가 각자의 입력에 서명해야 함");
        
        Ok(tx)
    }
    
    /// 정산 트랜잭션 생성 (만기시 실행)
    pub fn create_settlement_tx(
        &self,
        option: &BitcoinOption,
        option_utxo: OutPoint,
        spot_price: u64,
        oracle_proof: Vec<u8>,
        verifier_key: &SecretKey,
    ) -> Result<Transaction> {
        let (taproot_script, spend_info) = option.create_taproot_script()?;
        
        // 정산 금액 계산
        let settlement_amount = match option.option_type {
            OptionType::Call => {
                if spot_price > option.strike_price {
                    option.collateral // ITM: 구매자가 받음
                } else {
                    0 // OTM: 판매자가 유지
                }
            }
            OptionType::Put => {
                if spot_price < option.strike_price {
                    option.collateral // ITM: 구매자가 받음
                } else {
                    0 // OTM: 판매자가 유지
                }
            }
        };
        
        // 입력
        let input = TxIn {
            previous_output: option_utxo,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::from_height(option.expiry_block as u16),
            witness: Witness::new(),
        };
        
        // 출력
        let output = if settlement_amount > 0 {
            // ITM: 구매자에게 지급
            TxOut {
                value: Amount::from_sat(option.premium + option.collateral - 1000), // 수수료 제외
                script_pubkey: {
                    let pubkey = PublicKey::new(option.buyer_pubkey);
                    let compressed = CompressedPublicKey::try_from(pubkey).unwrap();
                    Address::p2wpkh(&compressed, self.network).script_pubkey()
                },
            }
        } else {
            // OTM: 판매자에게 반환
            TxOut {
                value: Amount::from_sat(option.premium + option.collateral - 1000),
                script_pubkey: {
                    let pubkey = PublicKey::new(option.seller_pubkey);
                    let compressed = CompressedPublicKey::try_from(pubkey).unwrap();
                    Address::p2wpkh(&compressed, self.network).script_pubkey()
                },
            }
        };
        
        let mut tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: LockTime::from_height(option.expiry_block).unwrap(),
            input: vec![input],
            output: vec![output],
        };
        
        // Script path witness 구성
        println!("⚠️  실제 배포시 필요:");
        println!("  1. Oracle 증명 데이터");
        println!("  2. 검증자 서명");
        println!("  3. Control block");
        println!("  4. Script revelation");
        
        Ok(tx)
    }
    
    /// Testnet 주소 생성
    pub fn generate_testnet_address(&self, secp_pubkey: &bitcoin::secp256k1::PublicKey) -> Address {
        let pubkey = PublicKey::new(*secp_pubkey);
        let compressed = CompressedPublicKey::try_from(pubkey).unwrap();
        Address::p2wpkh(&compressed, self.network)
    }
    
    /// Taproot 주소 생성
    pub fn generate_taproot_address(&self, option: &BitcoinOption) -> Result<Address> {
        let (script, _) = option.create_taproot_script()?;
        Ok(Address::from_script(&script, self.network)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::rand::thread_rng;
    use bitcoin::{Txid, hashes::Hash};
    use std::str::FromStr;
    
    #[test]
    fn test_create_funding_tx() {
        let deployer = TestnetDeployer::new();
        let mut rng = thread_rng();
        
        // 테스트 키 생성
        let buyer_key = SecretKey::new(&mut rng);
        let seller_key = SecretKey::new(&mut rng);
        let verifier_key = SecretKey::new(&mut rng);
        
        let secp = Secp256k1::new();
        
        // 옵션 생성
        let option = BitcoinOption {
            option_type: OptionType::Call,
            strike_price: 50_000_000_000,
            expiry_block: 850_000,
            buyer_pubkey: PublicKey::from_secret_key(&secp, &buyer_key),
            seller_pubkey: PublicKey::from_secret_key(&secp, &seller_key),
            verifier_pubkey: PublicKey::from_secret_key(&secp, &verifier_key),
            premium: 1_000_000,
            collateral: 10_000_000,
        };
        
        // 더미 UTXO
        let buyer_utxo = OutPoint {
            txid: Txid::from_str("0000000000000000000000000000000000000000000000000000000000000001").unwrap(),
            vout: 0,
        };
        let seller_utxo = OutPoint {
            txid: Txid::from_str("0000000000000000000000000000000000000000000000000000000000000002").unwrap(),
            vout: 0,
        };
        
        // 트랜잭션 생성
        let tx = deployer.create_option_funding_tx(
            &option,
            buyer_utxo,
            Amount::from_sat(2_000_000),
            seller_utxo,
            Amount::from_sat(15_000_000),
            &buyer_key,
            &seller_key,
        ).unwrap();
        
        // 검증
        assert_eq!(tx.input.len(), 2);
        assert_eq!(tx.output.len(), 3);
        assert_eq!(tx.output[0].value, Amount::from_sat(11_000_000)); // 프리미엄 + 담보
    }
}
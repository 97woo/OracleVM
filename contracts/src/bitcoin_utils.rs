use anyhow::Result;
use bitcoin::opcodes::all::{OP_CHECKSIG, OP_DROP, OP_ELSE, OP_ENDIF, OP_EQUAL, OP_IF};
use bitcoin::{
    absolute::LockTime,
    script::Builder,
    secp256k1::Secp256k1,
    taproot::{TaprootBuilder, TaprootSpendInfo},
    transaction::Version,
    Address, Amount, Network, OutPoint, PrivateKey, PublicKey, ScriptBuf, Transaction, TxIn, TxOut,
    Witness,
};

/// Taproot 주소 생성 유틸리티
pub struct TaprootAddressBuilder {
    secp: Secp256k1<bitcoin::secp256k1::All>,
    network: Network,
}

impl TaprootAddressBuilder {
    pub fn new(network: Network) -> Self {
        Self {
            secp: Secp256k1::new(),
            network,
        }
    }

    /// 옵션 컨트랙트용 Taproot 주소 생성
    pub fn create_option_contract_address(
        &self,
        user_pubkey: PublicKey,
        pool_pubkey: PublicKey,
        bitvmx_commitment: [u8; 32],
        expiry_height: u32,
    ) -> Result<(Address, TaprootSpendInfo)> {
        // 메인 스크립트 (BitVMX 증명 검증)
        let main_script = Builder::new()
            .push_slice(&bitvmx_commitment)
            .push_opcode(OP_EQUAL)
            .push_opcode(OP_IF)
            .push_key(&pool_pubkey)
            .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_ELSE)
            .push_int(expiry_height as i64)
            .push_opcode(OP_DROP)
            .push_key(&user_pubkey)
            .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_ENDIF)
            .into_script();

        // Taproot 트리 생성
        let taproot_builder = TaprootBuilder::new().add_leaf(0, main_script)?;

        let tapinfo = taproot_builder.finalize(&self.secp, pool_pubkey.inner)?;

        let address = Address::p2tr_tweaked(tapinfo.output_key(), self.network);

        Ok((address, tapinfo))
    }

    /// 유동성 풀 주소 생성 (멀티시그)
    pub fn create_pool_address(
        &self,
        operator_pubkey: PublicKey,
        guardian_pubkeys: Vec<PublicKey>,
        threshold: usize,
    ) -> Result<Address> {
        // 2-of-3 멀티시그 예시
        let multisig_script = Builder::new()
            .push_int(threshold as i64)
            .push_key(&operator_pubkey);

        let mut builder = multisig_script;
        for pubkey in &guardian_pubkeys {
            builder = builder.push_key(pubkey);
        }

        let script = builder
            .push_int((guardian_pubkeys.len() + 1) as i64)
            .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
            .into_script();

        // P2WSH 주소로 변환
        Ok(Address::p2wsh(&script, self.network))
    }
}

/// 트랜잭션 생성 유틸리티
pub struct TransactionBuilder {
    network: Network,
}

impl TransactionBuilder {
    pub fn new(network: Network) -> Self {
        Self { network }
    }

    /// 옵션 구매 트랜잭션 생성
    pub fn build_option_purchase_tx(
        &self,
        user_utxo: OutPoint,
        user_amount: Amount,
        premium: Amount,
        option_address: Address,
        pool_address: Address,
        change_address: Address,
        fee: Amount,
    ) -> Result<Transaction> {
        let mut tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: user_utxo,
                script_sig: ScriptBuf::new(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            }],
            output: vec![],
        };

        // 옵션 컨트랙트 출력 (담보금)
        tx.output.push(TxOut {
            value: premium,
            script_pubkey: option_address.script_pubkey(),
        });

        // 풀로 프리미엄 지급
        tx.output.push(TxOut {
            value: premium,
            script_pubkey: pool_address.script_pubkey(),
        });

        // 잔돈
        let change = user_amount - premium - premium - fee;
        if change > bitcoin::Amount::from_sat(546) {
            // dust limit
            tx.output.push(TxOut {
                value: change,
                script_pubkey: change_address.script_pubkey(),
            });
        }

        Ok(tx)
    }

    /// 정산 트랜잭션 생성
    pub fn build_settlement_tx(
        &self,
        option_utxo: OutPoint,
        option_amount: Amount,
        settlement_amount: Amount,
        user_address: Address,
        pool_address: Address,
        bitvmx_proof: Vec<u8>,
        fee: Amount,
    ) -> Result<Transaction> {
        let mut tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: option_utxo,
                script_sig: ScriptBuf::new(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::from_slice(&[bitvmx_proof]),
            }],
            output: vec![],
        };

        // 사용자에게 정산금 지급
        if settlement_amount > Amount::ZERO {
            tx.output.push(TxOut {
                value: settlement_amount,
                script_pubkey: user_address.script_pubkey(),
            });
        }

        // 잔액은 풀로 반환
        let remaining = option_amount - settlement_amount - fee;
        if remaining > bitcoin::Amount::from_sat(546) {
            tx.output.push(TxOut {
                value: remaining,
                script_pubkey: pool_address.script_pubkey(),
            });
        }

        Ok(tx)
    }
}

/// Bitcoin 유틸리티 함수들
pub fn create_taproot_address(pubkey: PublicKey, network: Network) -> Address {
    Address::p2tr(&Secp256k1::new(), pubkey.inner, None, network)
}

pub fn build_transaction(inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Transaction {
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: inputs,
        output: outputs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::rand::thread_rng;

    #[test]
    fn test_taproot_address_creation() {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut thread_rng());
        let pubkey =
            PublicKey::from_private_key(&secp, &PrivateKey::new(secret_key, Network::Testnet));

        let address = create_taproot_address(pubkey, Network::Testnet);

        assert!(address.to_string().starts_with("tb1p"));
    }

    #[test]
    fn test_option_contract_address() {
        let builder = TaprootAddressBuilder::new(Network::Testnet);
        let secp = Secp256k1::new();

        let (_, user_pubkey) = secp.generate_keypair(&mut thread_rng());
        let (_, pool_pubkey) = secp.generate_keypair(&mut thread_rng());

        let user_pk = PublicKey::from_slice(&user_pubkey.serialize()).unwrap();
        let pool_pk = PublicKey::from_slice(&pool_pubkey.serialize()).unwrap();
        let bitvmx_commitment = [0u8; 32];

        let result = builder.create_option_contract_address(
            user_pk,
            pool_pk,
            bitvmx_commitment,
            800000, // 블록 높이
        );

        assert!(result.is_ok());
        let (address, _) = result.unwrap();
        assert!(address.to_string().starts_with("tb1p"));
    }
}

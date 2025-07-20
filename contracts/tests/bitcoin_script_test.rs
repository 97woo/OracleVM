use anyhow::Result;
use bitcoin::blockdata::script::Builder;
use bitcoin::ScriptBuf;
use bitcoin::blockdata::opcodes;
use bitcoin::PublicKey;
use btcfi_contracts::OptionType;

/// 옵션 컨트랙트 스크립트 파라미터
#[derive(Debug, Clone)]
pub struct OptionScriptParams {
    pub buyer_pubkey: PublicKey,
    pub seller_pubkey: PublicKey,
    pub oracle_pubkey: PublicKey,
    pub strike_price: u64,
    pub expiry_height: u32,
    pub option_type: OptionType,
}

/// Call 옵션 스크립트 생성
/// 만기 시: Oracle이 가격 증명 제출 → ITM이면 구매자가 인출, OTM이면 판매자가 회수
pub fn create_call_option_script(params: &OptionScriptParams) -> ScriptBuf {
    Builder::new()
        // Case 1: 만기 후 Oracle 가격 증명으로 정산
        .push_opcode(opcodes::all::OP_IF)
            // Oracle 서명 확인
            .push_slice(params.oracle_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIGVERIFY)
            
            // 만기 확인
            .push_int(params.expiry_height as i64)
            .push_opcode(opcodes::all::OP_CLTV)
            .push_opcode(opcodes::all::OP_DROP)
            
            // 스택에서 spot price 가져오기
            .push_opcode(opcodes::all::OP_DUP)
            .push_int(params.strike_price as i64)
            
            // Call ITM 확인: spot > strike
            .push_opcode(opcodes::all::OP_GREATERTHAN)
            .push_opcode(opcodes::all::OP_IF)
                // ITM: 구매자가 받음
                .push_slice(params.buyer_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_ELSE)
                // OTM: 판매자가 회수
                .push_slice(params.seller_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_ENDIF)
            .push_opcode(opcodes::all::OP_CHECKSIG)
            
        // Case 2: 만기 전 양자 합의로 조기 종료
        .push_opcode(opcodes::all::OP_ELSE)
            .push_slice(params.buyer_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIGVERIFY)
            .push_slice(params.seller_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIG)
        .push_opcode(opcodes::all::OP_ENDIF)
        .into_script()
}

/// Put 옵션 스크립트 생성
pub fn create_put_option_script(params: &OptionScriptParams) -> ScriptBuf {
    Builder::new()
        // Case 1: 만기 후 Oracle 가격 증명으로 정산
        .push_opcode(opcodes::all::OP_IF)
            // Oracle 서명 확인
            .push_slice(params.oracle_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIGVERIFY)
            
            // 만기 확인
            .push_int(params.expiry_height as i64)
            .push_opcode(opcodes::all::OP_CLTV)
            .push_opcode(opcodes::all::OP_DROP)
            
            // 스택에서 spot price 가져오기
            .push_opcode(opcodes::all::OP_DUP)
            .push_int(params.strike_price as i64)
            
            // Put ITM 확인: spot < strike
            .push_opcode(opcodes::all::OP_LESSTHAN)
            .push_opcode(opcodes::all::OP_IF)
                // ITM: 구매자가 받음
                .push_slice(params.buyer_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_ELSE)
                // OTM: 판매자가 회수
                .push_slice(params.seller_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_ENDIF)
            .push_opcode(opcodes::all::OP_CHECKSIG)
            
        // Case 2: 만기 전 양자 합의로 조기 종료
        .push_opcode(opcodes::all::OP_ELSE)
            .push_slice(params.buyer_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIGVERIFY)
            .push_slice(params.seller_pubkey.to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIG)
        .push_opcode(opcodes::all::OP_ENDIF)
        .into_script()
}

/// 유동성 풀 스크립트 생성
/// 멀티시그 또는 타임락 조건으로 관리
pub fn create_liquidity_pool_script(
    pool_managers: &[PublicKey],
    threshold: usize,
    emergency_timeout: u32,
) -> ScriptBuf {
    Builder::new()
        // Case 1: 정상 운영 - M of N 멀티시그
        .push_opcode(opcodes::all::OP_IF)
            .push_int(threshold as i64)
            // 각 관리자 공개키 추가
            .push_slice(pool_managers[0].to_bytes())
            .push_slice(pool_managers[1].to_bytes())
            .push_slice(pool_managers[2].to_bytes())
            .push_int(pool_managers.len() as i64)
            .push_opcode(opcodes::all::OP_CHECKMULTISIG)
            
        // Case 2: 비상 타임아웃 - 모든 관리자가 회수 가능
        .push_opcode(opcodes::all::OP_ELSE)
            .push_int(emergency_timeout as i64)
            .push_opcode(opcodes::all::OP_CLTV)
            .push_opcode(opcodes::all::OP_DROP)
            // 아무 관리자나 서명으로 회수
            .push_slice(pool_managers[0].to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIG)
            .push_slice(pool_managers[1].to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIG)
            .push_opcode(opcodes::all::OP_BOOLOR)
            .push_slice(pool_managers[2].to_bytes())
            .push_opcode(opcodes::all::OP_CHECKSIG)
            .push_opcode(opcodes::all::OP_BOOLOR)
        .push_opcode(opcodes::all::OP_ENDIF)
        .into_script()
}

/// 정산 증명 스크립트 (Oracle이 가격 데이터 커밋)
pub fn create_settlement_commitment_script(
    oracle_pubkey: PublicKey,
    price_commitment_hash: &[u8; 32],
) -> ScriptBuf {
    Builder::new()
        // Oracle 서명 확인
        .push_slice(oracle_pubkey.to_bytes())
        .push_opcode(opcodes::all::OP_CHECKSIGVERIFY)
        
        // 가격 데이터 해시 확인
        .push_opcode(opcodes::all::OP_HASH256)
        .push_slice(price_commitment_hash)
        .push_opcode(opcodes::all::OP_EQUAL)
        .into_script()
}

/// 스크립트 크기 검증
pub fn validate_script_size(script: &ScriptBuf) -> Result<()> {
    const MAX_SCRIPT_SIZE: usize = 10_000; // Bitcoin 표준 제한
    
    if script.len() > MAX_SCRIPT_SIZE {
        anyhow::bail!("Script size {} exceeds maximum {}", script.len(), MAX_SCRIPT_SIZE);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Secp256k1, SecretKey};
    use bitcoin::PublicKey;

    fn generate_test_pubkey(seed: u8) -> PublicKey {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[seed; 32]).unwrap();
        PublicKey::from_private_key(&secp, &secret_key.into())
    }

    #[test]
    fn test_create_call_option_script() {
        // Given
        let params = OptionScriptParams {
            buyer_pubkey: generate_test_pubkey(1),
            seller_pubkey: generate_test_pubkey(2),
            oracle_pubkey: generate_test_pubkey(3),
            strike_price: 70_000_00, // $70,000 in cents
            expiry_height: 800_000,
            option_type: OptionType::Call,
        };

        // When
        let script = create_call_option_script(&params);

        // Then
        assert!(validate_script_size(&script).is_ok());
        assert!(script.len() > 0);
        
        // 스크립트에 필요한 요소들이 포함되어 있는지 확인
        let script_bytes = script.as_bytes();
        assert!(script_bytes.windows(33).any(|w| w == params.buyer_pubkey.to_bytes()));
        assert!(script_bytes.windows(33).any(|w| w == params.seller_pubkey.to_bytes()));
        assert!(script_bytes.windows(33).any(|w| w == params.oracle_pubkey.to_bytes()));
    }

    #[test]
    fn test_create_put_option_script() {
        // Given
        let params = OptionScriptParams {
            buyer_pubkey: generate_test_pubkey(1),
            seller_pubkey: generate_test_pubkey(2),
            oracle_pubkey: generate_test_pubkey(3),
            strike_price: 70_000_00,
            expiry_height: 800_000,
            option_type: OptionType::Put,
        };

        // When
        let script = create_put_option_script(&params);

        // Then
        assert!(validate_script_size(&script).is_ok());
        
        // Put 옵션은 LESSTHAN opcode를 사용
        let script_bytes = script.as_bytes();
        assert!(script_bytes.iter().any(|&b| b == opcodes::all::OP_LESSTHAN.to_u8()));
    }

    #[test]
    fn test_create_liquidity_pool_script() {
        // Given
        let managers = vec![
            generate_test_pubkey(1),
            generate_test_pubkey(2),
            generate_test_pubkey(3),
        ];
        let threshold = 2; // 2 of 3 multisig
        let emergency_timeout = 850_000;

        // When
        let script = create_liquidity_pool_script(&managers, threshold, emergency_timeout);

        // Then
        assert!(validate_script_size(&script).is_ok());
        
        // 멀티시그 opcode 확인
        let script_bytes = script.as_bytes();
        assert!(script_bytes.iter().any(|&b| b == opcodes::all::OP_CHECKMULTISIG.to_u8()));
    }

    #[test]
    fn test_create_settlement_commitment_script() {
        // Given
        let oracle_pubkey = generate_test_pubkey(1);
        let price_commitment_hash = [0xAB; 32];

        // When
        let script = create_settlement_commitment_script(oracle_pubkey, &price_commitment_hash);

        // Then
        assert!(validate_script_size(&script).is_ok());
        
        // HASH256 opcode 확인
        let script_bytes = script.as_bytes();
        assert!(script_bytes.iter().any(|&b| b == opcodes::all::OP_HASH256.to_u8()));
    }

    #[test]
    fn test_script_size_validation() {
        // Given - 매우 큰 스크립트 생성
        let mut builder = Builder::new();
        for _ in 0..500 {
            builder = builder.push_slice(&[0xFF; 200]);
        }
        let large_script = builder.into_script();

        // When
        let result = validate_script_size(&large_script);

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[test]
    fn test_option_script_with_timelock() {
        // Given
        let params = OptionScriptParams {
            buyer_pubkey: generate_test_pubkey(1),
            seller_pubkey: generate_test_pubkey(2),
            oracle_pubkey: generate_test_pubkey(3),
            strike_price: 70_000_00,
            expiry_height: 800_000,
            option_type: OptionType::Call,
        };

        // When
        let script = create_call_option_script(&params);

        // Then - CHECKLOCKTIMEVERIFY opcode 확인
        let script_bytes = script.as_bytes();
        assert!(script_bytes.iter().any(|&b| b == opcodes::all::OP_CLTV.to_u8()));
    }

    #[test]
    fn test_multisig_threshold_encoding() {
        // Given
        let managers = vec![
            generate_test_pubkey(1),
            generate_test_pubkey(2),
            generate_test_pubkey(3),
        ];

        // When - Different thresholds
        for threshold in 1..=3 {
            let script = create_liquidity_pool_script(&managers, threshold, 850_000);
            
            // Then
            assert!(validate_script_size(&script).is_ok());
            
            // 임계값이 스크립트에 포함되어 있는지 확인
            let script_bytes = script.as_bytes();
            assert!(script_bytes.iter().any(|&b| b == (opcodes::all::OP_PUSHNUM_1.to_u8() + threshold as u8 - 1)));
        }
    }

    #[test]
    fn test_emergency_recovery_path() {
        // Given
        let managers = vec![
            generate_test_pubkey(1),
            generate_test_pubkey(2),
            generate_test_pubkey(3),
        ];
        let emergency_timeout = 850_000;

        // When
        let script = create_liquidity_pool_script(&managers, 2, emergency_timeout);

        // Then - BOOLOR opcodes for any manager recovery
        let script_bytes = script.as_bytes();
        let boolor_count = script_bytes.iter().filter(|&&b| b == opcodes::all::OP_BOOLOR.to_u8()).count();
        assert_eq!(boolor_count, 2); // For 3 managers, need 2 BOOLOR
    }

    #[test]
    fn test_price_commitment_verification() {
        // Given
        let oracle_pubkey = generate_test_pubkey(1);
        let price_data = b"BTC:70000,ETH:3500,timestamp:1700000000";
        let commitment_hash = bitcoin::hashes::sha256d::Hash::hash(price_data);

        // When
        let script = create_settlement_commitment_script(oracle_pubkey, commitment_hash.as_ref());

        // Then
        assert!(validate_script_size(&script).is_ok());
        
        // 해시가 스크립트에 포함되어 있는지 확인
        let script_bytes = script.as_bytes();
        assert!(script_bytes.windows(32).any(|w| w == commitment_hash.as_ref()));
    }
}
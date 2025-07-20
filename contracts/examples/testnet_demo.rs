use btcfi_contracts::bitcoin_option::{BitcoinOption, OptionType};
use btcfi_contracts::testnet_deployer::TestnetDeployer;
use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey};
use bitcoin::{Network, Transaction, Address};
use anyhow::Result;
use std::str::FromStr;

/// Bitcoin Testnet에서 실제 옵션 데모
/// 
/// 이 예제는 실제 Testnet에서 옵션을 생성하고 테스트하는 방법을 보여줍니다.
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Bitcoin Testnet 옵션 데모");
    println!("================================\n");
    
    let secp = Secp256k1::new();
    let deployer = TestnetDeployer::new();
    
    // 테스트용 키 (실제로는 generate-keys로 생성한 키 사용)
    let buyer_secret = SecretKey::from_str("d8a1e1224e63135765bde9dc8a2c8e403eee8be73d3589d58c5ddbf9dce3fdf4")?;
    let seller_secret = SecretKey::from_str("143c9cf988b64adb053a0ee3ef7e3bfb5a3c424e0112c606df4d158ef0e59f2f")?;
    let verifier_secret = SecretKey::from_str("3e3d98605246602c99a7b29f251f1b7a761c398dec3ebbee7cba2a4827a710ef")?;
    
    let buyer_pubkey = PublicKey::from_secret_key(&secp, &buyer_secret);
    let seller_pubkey = PublicKey::from_secret_key(&secp, &seller_secret);
    let verifier_pubkey = PublicKey::from_secret_key(&secp, &verifier_secret);
    
    // 1. 참여자 정보 표시
    println!("📋 참여자 정보:");
    println!("  구매자 주소: {}", deployer.generate_testnet_address(&buyer_pubkey));
    println!("  판매자 주소: {}", deployer.generate_testnet_address(&seller_pubkey));
    println!("  검증자 주소: {}\n", deployer.generate_testnet_address(&verifier_pubkey));
    
    // 2. 옵션 파라미터 설정
    let option = BitcoinOption {
        option_type: OptionType::Call,
        strike_price: 50_000_000_000, // $50k in satoshis
        expiry_block: 2_580_000, // 약 1주일 후
        buyer_pubkey,
        seller_pubkey,
        verifier_pubkey,
        premium: 1_000_000, // 0.01 BTC
        collateral: 10_000_000, // 0.1 BTC
    };
    
    println!("📄 옵션 상세:");
    println!("  타입: Call Option");
    println!("  행사가: $50,000");
    println!("  프리미엄: 0.01 BTC");
    println!("  담보: 0.1 BTC");
    println!("  만기 블록: {}\n", option.expiry_block);
    
    // 3. Taproot 옵션 주소 생성
    let option_address = deployer.generate_taproot_address(&option)?;
    println!("🏦 옵션 컨트랙트 주소:");
    println!("  {}", option_address);
    println!("  https://mempool.space/testnet/address/{}\n", option_address);
    
    // 4. 펀딩 지침
    println!("💰 펀딩 지침:");
    println!("  1. 구매자: {} BTC를 옵션 주소로 전송", 0.01);
    println!("  2. 판매자: {} BTC를 옵션 주소로 전송", 0.1);
    println!("  3. 총 {} BTC가 옵션 주소에 잠김\n", 0.11);
    
    // 5. 만기 시나리오
    println!("📊 만기 시나리오:");
    println!("  현재 블록: ~2,570,000 (예상)");
    println!("  만기 블록: {}", option.expiry_block);
    println!("  남은 블록: ~{} (약 {} 시간)\n", 
        option.expiry_block - 2_570_000, 
        (option.expiry_block - 2_570_000) / 6
    );
    
    // 6. 정산 프로세스
    println!("⚡ 정산 프로세스:");
    println!("  1. 만기 블록 도달 시 Oracle이 BTC 가격 수집");
    println!("  2. BitVMX가 정산 증명 생성");
    println!("  3. 다음 중 하나 실행:");
    println!("     - ITM (BTC > $50k): 구매자가 0.11 BTC 수령");
    println!("     - OTM (BTC < $50k): 판매자가 0.11 BTC 회수\n");
    
    // 7. 모니터링 도구
    println!("🔍 모니터링 도구:");
    println!("  - Mempool: https://mempool.space/testnet");
    println!("  - Blockstream: https://blockstream.info/testnet");
    println!("  - 현재 블록: bitcoin-cli -testnet getblockcount\n");
    
    // 8. 스크립트 분석 (디버깅용)
    let (script, spend_info) = option.create_taproot_script()?;
    println!("🔧 기술적 세부사항:");
    println!("  Taproot 스크립트 크기: {} bytes", script.len());
    println!("  Merkle root: {:?}", spend_info.merkle_root());
    println!("  Script tree depth: 1 (settlement + refund)\n");
    
    println!("✅ 준비 완료! 위 주소로 자금을 전송하여 옵션을 활성화하세요.");
    println!("📌 주의: 실제 Testnet BTC가 필요합니다!");
    
    Ok(())
}

// 실행 방법:
// cargo run --example testnet_demo
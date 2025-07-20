use btcfi_contracts::bitcoin_option::{BitcoinOption, OptionType};
use btcfi_contracts::bitvmx_bridge::BitVmxBridge;
use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey};
use bitcoin::secp256k1::rand::thread_rng;
use anyhow::Result;

/// Bitcoin L1 단방향 옵션 예제
/// 
/// 이 예제는 다음을 보여줍니다:
/// 1. Bitcoin L1에서 직접 실행되는 옵션 컨트랙트 생성
/// 2. Taproot를 통한 조건부 정산 스크립트
/// 3. BitVMX를 통한 오프체인 계산과 온체인 검증
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Bitcoin L1 단방향 옵션 시스템 데모");
    println!("=====================================\n");
    
    let secp = Secp256k1::new();
    let mut rng = thread_rng();
    
    // 1. 참여자 키 생성
    let buyer_key = SecretKey::new(&mut rng);
    let seller_key = SecretKey::new(&mut rng);
    let verifier_key = SecretKey::new(&mut rng);
    
    let buyer_pubkey = PublicKey::from_secret_key(&secp, &buyer_key);
    let seller_pubkey = PublicKey::from_secret_key(&secp, &seller_key);
    let verifier_pubkey = PublicKey::from_secret_key(&secp, &verifier_key);
    
    println!("📌 참여자 공개키:");
    println!("  - 구매자: {:?}", hex::encode(&buyer_pubkey.serialize()));
    println!("  - 판매자: {:?}", hex::encode(&seller_pubkey.serialize()));
    println!("  - 검증자: {:?}", hex::encode(&verifier_pubkey.serialize()));
    println!();
    
    // 2. 콜 옵션 생성 (Strike: $50k, Premium: 0.01 BTC, Collateral: 0.1 BTC)
    let option = BitcoinOption {
        option_type: OptionType::Call,
        strike_price: 50_000_000_000, // $50k in satoshis (assuming 1 BTC = $100k)
        expiry_block: 850_000,
        buyer_pubkey,
        seller_pubkey,
        verifier_pubkey,
        premium: 1_000_000, // 0.01 BTC
        collateral: 10_000_000, // 0.1 BTC
    };
    
    println!("📄 옵션 상세:");
    println!("  - 타입: Call Option");
    println!("  - 행사가: $50,000");
    println!("  - 프리미엄: 0.01 BTC");
    println!("  - 담보: 0.1 BTC");
    println!("  - 만기 블록: {}", option.expiry_block);
    println!();
    
    // 3. Taproot 스크립트 생성
    let (taproot_script, spend_info) = option.create_taproot_script()?;
    println!("✅ Taproot 옵션 스크립트 생성 완료");
    println!("  - 스크립트 크기: {} bytes", taproot_script.len());
    println!("  - P2TR 주소로 자금 전송 필요");
    println!();
    
    // 4. 시나리오: 만기시 가격 정산
    println!("📊 만기 시나리오 시뮬레이션:");
    println!("=====================================\n");
    
    // BitVMX 브릿지 초기화
    let bridge = BitVmxBridge::new();
    
    // 시나리오 1: ITM (In The Money) - Spot $52k
    println!("1️⃣ ITM 시나리오: Spot Price = $52,000");
    let spot_itm = 52_000_000_000; // $52k in satoshis
    
    let input_itm = bridge.prepare_settlement_input(&option, spot_itm);
    println!("  - BitVMX 입력: {}", hex::encode(&input_itm));
    
    // 실제로는 BitVMX가 증명을 생성하지만, 여기서는 시뮬레이션
    let settlement_amount_itm = if spot_itm > option.strike_price {
        option.collateral // 구매자가 담보 전액 수령
    } else {
        0
    };
    
    println!("  - 정산 결과: 구매자가 {} sats 수령", settlement_amount_itm);
    println!("  - 수익률: {}%", (settlement_amount_itm as f64 / option.premium as f64 - 1.0) * 100.0);
    println!();
    
    // 시나리오 2: OTM (Out of The Money) - Spot $48k
    println!("2️⃣ OTM 시나리오: Spot Price = $48,000");
    let spot_otm = 48_000_000_000; // $48k in satoshis
    
    let input_otm = bridge.prepare_settlement_input(&option, spot_otm);
    println!("  - BitVMX 입력: {}", hex::encode(&input_otm));
    
    let settlement_amount_otm = if spot_otm > option.strike_price {
        option.collateral
    } else {
        0 // 판매자가 담보 유지
    };
    
    println!("  - 정산 결과: 판매자가 담보 {} sats 유지", option.collateral);
    println!("  - 구매자 손실: {} sats (프리미엄)", option.premium);
    println!();
    
    // 5. 온체인 검증 프로세스 설명
    println!("🔍 온체인 검증 프로세스:");
    println!("=====================================\n");
    println!("1. Oracle들이 만기 시점 BTC 가격 수집");
    println!("2. BitVMX가 오프체인에서 정산 금액 계산");
    println!("3. 검증자가 BitVMX 증명과 서명 생성");
    println!("4. Bitcoin Script가 자동으로 검증 및 정산 실행:");
    println!("   - 시간 잠금 확인 (블록 {})", option.expiry_block);
    println!("   - BitVMX 증명 해시 검증");
    println!("   - 검증자 서명 확인");
    println!("   - ITM/OTM에 따라 자금 이동");
    println!();
    
    println!("✨ Bitcoin L1 네이티브 옵션의 장점:");
    println!("  - 신뢰 최소화: 스마트 컨트랙트 없이 Bitcoin Script로 실행");
    println!("  - 자동 정산: 만기시 자동으로 정산 (중개자 불필요)");
    println!("  - 투명성: 모든 조건이 온체인에 공개");
    println!("  - 보안성: Bitcoin의 보안성 그대로 활용");
    
    Ok(())
}

// 실행 방법:
// cargo run --example bitcoin_option_example
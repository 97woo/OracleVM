use btcfi_contracts::{OptionType, SimpleContractManager};

#[tokio::test]
async fn test_full_option_lifecycle() {
    let mut manager = SimpleContractManager::new();

    // 1. 유동성 추가: 1 BTC
    manager.add_liquidity(100_000_000).unwrap();

    // 2. Call 옵션 생성
    manager
        .create_option(
            "CALL-TEST".to_string(),
            OptionType::Call,
            7_000_000,  // $70,000 strike in cents
            10_000_000, // 0.1 BTC quantity in sats
            250_000,    // 0.0025 BTC premium in sats
            800_000,    // expiry height
            "user1".to_string(),
        )
        .unwrap();

    // 3. 상태 확인
    assert_eq!(manager.pool_state.active_options, 1);
    assert_eq!(manager.pool_state.locked_collateral, 10_000_000);
    assert_eq!(manager.pool_state.total_premium_collected, 250_000);

    // 4. Put 옵션 생성
    manager
        .create_option(
            "PUT-TEST".to_string(),
            OptionType::Put,
            6_500_000,  // $65,000 strike
            20_000_000, // 0.2 BTC quantity
            180_000,    // 0.0018 BTC premium
            800_000,
            "user2".to_string(),
        )
        .unwrap();

    assert_eq!(manager.pool_state.active_options, 2);

    // 5. Call 옵션 정산 (ITM - Spot $72,000)
    let call_payout = manager.settle_option("CALL-TEST", 7_200_000).unwrap();
    assert!(call_payout > 0);

    // 6. Put 옵션 정산 (ITM - Spot $63,000)
    let put_payout = manager.settle_option("PUT-TEST", 6_300_000).unwrap();
    assert!(put_payout > 0);

    // 7. 최종 상태 확인
    assert_eq!(manager.pool_state.active_options, 0);

    let system_status = manager.get_system_status();
    println!("✅ Full option lifecycle test passed");
    println!("   System status: {}", system_status);
    println!("   Call payout: {} sats", call_payout);
    println!("   Put payout: {} sats", put_payout);
    println!(
        "   Utilization rate: {:.2}%",
        manager.pool_state.utilization_rate()
    );
}

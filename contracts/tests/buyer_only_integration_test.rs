use btcfi_contracts::{
    BuyerOnlyOptionManager, AggregatedPrice, 
    OptionType
};
use btcfi_contracts::buyer_only_option::OptionStatus;

#[test]
fn test_buyer_only_option_full_lifecycle() {
    // 1. Initialize pool with 1 BTC liquidity
    let mut manager = BuyerOnlyOptionManager::new(100_000_000); // 1 BTC
    
    // 2. Set current price (3-exchange aggregation)
    let current_price = AggregatedPrice {
        binance_price: 7000000,  // $70,000
        coinbase_price: 7005000, // $70,050
        kraken_price: 6995000,   // $69,950
        average_price: 7000000,  // $70,000
        timestamp: 1234567890,
    };
    manager.update_price(current_price);
    
    // 3. Buy call option with target theta
    let option_result = manager.buy_option(
        OptionType::Call,
        7500000,            // $75,000 strike
        1_000_000,          // 0.01 BTC notional
        -0.02,              // Target theta: -2% daily decay
        7.0,                // 7 days to expiry
        "bc1qbuyer123".to_string(),
    );
    
    assert!(option_result.is_ok());
    let option = option_result.unwrap();
    
    // Verify option properties
    assert_eq!(option.option_type, OptionType::Call);
    assert_eq!(option.strike_price, 7500000);
    assert_eq!(option.quantity, 1_000_000);
    assert!(option.premium_paid > 0);
    assert_eq!(option.target_theta, -0.02);
    assert!(option.implied_volatility > 0.0);
    assert_eq!(option.status, OptionStatus::Active);
    
    // 4. Check pool state after purchase
    let pool_stats = manager.get_pool_stats();
    assert_eq!(pool_stats.total_premium_collected, option.premium_paid);
    assert!(pool_stats.available_liquidity < 100_000_000);
    assert!(pool_stats.locked_for_payouts > 0);
    assert!(pool_stats.net_delta.abs() > 0.0); // Should have delta exposure
    assert_eq!(pool_stats.net_theta, -0.02); // Target theta
    
    // 5. Settle option ITM
    let settlement_price = 8000000; // $80,000 (ITM)
    let payout = manager.settle_option(&option.option_id, settlement_price).unwrap();
    
    // Verify ITM payout
    assert!(payout > 0);
    let expected_payout = ((settlement_price - option.strike_price) as u64 * option.quantity) / settlement_price;
    assert_eq!(payout, expected_payout);
    
    // 6. Verify final pool state
    let final_pool = manager.get_pool_stats();
    assert_eq!(final_pool.total_payouts, payout);
    assert_eq!(final_pool.active_options.len(), 0);
    assert_eq!(final_pool.net_theta, 0.0); // No active options
}

#[test]
fn test_buyer_only_option_otm_expiry() {
    let mut manager = BuyerOnlyOptionManager::new(100_000_000);
    
    manager.update_price(AggregatedPrice {
        binance_price: 7000000,
        coinbase_price: 7000000,
        kraken_price: 7000000,
        average_price: 7000000,
        timestamp: 1234567890,
    });
    
    // Buy put option
    let option = manager.buy_option(
        OptionType::Put,
        6500000,            // $65,000 strike
        2_000_000,          // 0.02 BTC notional
        -0.015,             // Target theta: -1.5% daily decay
        3.0,                // 3 days to expiry
        "bc1qbuyer456".to_string(),
    ).unwrap();
    
    let premium_paid = option.premium_paid;
    
    // Settle OTM
    let settlement_price = 7000000; // $70,000 (OTM for put)
    let payout = manager.settle_option(&option.option_id, settlement_price).unwrap();
    
    // Verify OTM result
    assert_eq!(payout, 0);
    
    // Pool should keep all premium as theta revenue
    let final_pool = manager.get_pool_stats();
    assert_eq!(final_pool.theta_revenue, premium_paid);
    assert_eq!(final_pool.total_payouts, 0);
}

#[test]
fn test_insufficient_liquidity() {
    let mut manager = BuyerOnlyOptionManager::new(100_000); // Only 0.001 BTC
    
    manager.update_price(AggregatedPrice {
        binance_price: 7000000,
        coinbase_price: 7000000,
        kraken_price: 7000000,
        average_price: 7000000,
        timestamp: 1234567890,
    });
    
    // Try to buy option with large notional
    let result = manager.buy_option(
        OptionType::Call,
        7000000,
        10_000_000,  // 0.1 BTC (exceeds pool liquidity)
        -0.02,
        7.0,
        "bc1qbuyer789".to_string(),
    );
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Insufficient liquidity"));
}

#[test]
fn test_delta_rebalancing_threshold() {
    let mut manager = BuyerOnlyOptionManager::new(1_000_000_000); // 10 BTC
    
    manager.update_price(AggregatedPrice {
        binance_price: 7000000,
        coinbase_price: 7000000,
        kraken_price: 7000000,
        average_price: 7000000,
        timestamp: 1234567890,
    });
    
    // Buy multiple options to accumulate delta
    for i in 0..5 {
        manager.buy_option(
            OptionType::Call,
            7000000 + (i * 100000), // Different strikes
            10_000_000,             // 0.1 BTC each
            -0.02,
            7.0,
            format!("bc1qbuyer{}", i),
        ).unwrap();
    }
    
    // Check accumulated delta
    let pool = manager.get_pool_stats();
    assert!(pool.net_delta.abs() > 0.1); // Should trigger rebalancing threshold
    
    // In production, this would trigger external hedge rebalancing
    println!("Net delta requiring hedge: {:.4}", pool.net_delta);
}
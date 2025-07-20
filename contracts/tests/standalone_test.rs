// 독립적인 테스트 모듈 - 외부 의존성 최소화
use btcfi_contracts::{OptionType, OptionStatus, SimpleOption, SimplePoolState};

#[test]
fn test_option_creation() {
    // Given
    let option = SimpleOption {
        option_id: "OPT-001".to_string(),
        option_type: OptionType::Call,
        strike_price: 7_000_000, // $70,000 in cents
        quantity: 10_000_000,    // 0.1 BTC
        premium_paid: 100_000,   // 0.001 BTC
        expiry_height: 801_000,
        status: OptionStatus::Active,
        user_id: "user123".to_string(),
    };

    // Then
    assert_eq!(option.option_id, "OPT-001");
    assert_eq!(option.option_type, OptionType::Call);
    assert_eq!(option.status, OptionStatus::Active);
}

#[test]
fn test_option_settlement_calculation() {
    // Given - Call option
    let call_option = SimpleOption {
        option_id: "CALL-001".to_string(),
        option_type: OptionType::Call,
        strike_price: 7_000_000, // $70,000
        quantity: 10_000_000,    // 0.1 BTC
        premium_paid: 100_000,
        expiry_height: 801_000,
        status: OptionStatus::Active,
        user_id: "user123".to_string(),
    };
    
    let spot_price = 7_500_000; // $75,000

    // When - Calculate ITM amount
    let is_itm = spot_price > call_option.strike_price;
    let payout = if is_itm {
        ((spot_price - call_option.strike_price) as u128 * call_option.quantity as u128 
            / spot_price as u128) as u64
    } else {
        0
    };

    // Then
    assert!(is_itm);
    assert_eq!(payout, 666_666); // ~0.00667 BTC
}

#[test]
fn test_put_option_settlement() {
    // Given - Put option
    let put_option = SimpleOption {
        option_id: "PUT-001".to_string(),
        option_type: OptionType::Put,
        strike_price: 7_000_000, // $70,000
        quantity: 10_000_000,    // 0.1 BTC
        premium_paid: 100_000,
        expiry_height: 801_000,
        status: OptionStatus::Active,
        user_id: "user123".to_string(),
    };
    
    let spot_price = 6_500_000; // $65,000

    // When - Calculate ITM amount
    let is_itm = spot_price < put_option.strike_price;
    let payout = if is_itm {
        ((put_option.strike_price - spot_price) as u128 * put_option.quantity as u128 
            / spot_price as u128) as u64
    } else {
        0
    };

    // Then
    assert!(is_itm);
    assert_eq!(payout, 769_230); // ~0.00769 BTC
}

#[test]
fn test_pool_state_management() {
    // Given
    let mut pool = SimplePoolState::new();

    // When - Add liquidity
    pool.total_liquidity = 100_000_000; // 1 BTC
    pool.available_liquidity = 100_000_000;

    // Lock collateral for option
    let collateral_needed = 10_000_000; // 0.1 BTC
    pool.locked_collateral += collateral_needed;
    pool.available_liquidity -= collateral_needed;
    pool.active_options += 1;

    // Then
    assert_eq!(pool.total_liquidity, 100_000_000);
    assert_eq!(pool.locked_collateral, 10_000_000);
    assert_eq!(pool.available_liquidity, 90_000_000);
    assert_eq!(pool.active_options, 1);
}

#[test]
fn test_pool_premium_collection() {
    // Given
    let mut pool = SimplePoolState::new();
    pool.total_liquidity = 100_000_000;
    pool.available_liquidity = 100_000_000;

    // When - Collect premium
    let premium = 1_000_000; // 0.01 BTC
    pool.total_liquidity += premium;
    pool.available_liquidity += premium;
    pool.total_premium_collected += premium;

    // Then
    assert_eq!(pool.total_liquidity, 101_000_000);
    assert_eq!(pool.available_liquidity, 101_000_000);
    assert_eq!(pool.total_premium_collected, 1_000_000);
}

#[test]
fn test_pool_settlement_payout() {
    // Given
    let mut pool = SimplePoolState::new();
    pool.total_liquidity = 100_000_000;
    pool.locked_collateral = 50_000_000;
    pool.available_liquidity = 50_000_000;

    // When - Payout ITM option
    let payout = 20_000_000; // 0.2 BTC
    pool.locked_collateral -= payout;
    pool.total_liquidity -= payout;
    pool.total_payout += payout;

    // Then
    assert_eq!(pool.locked_collateral, 30_000_000);
    assert_eq!(pool.total_liquidity, 80_000_000);
    assert_eq!(pool.total_payout, 20_000_000);
}

#[test]
fn test_option_validation() {
    // Test strike price boundaries
    assert!(validate_strike_price(0).is_err());
    assert!(validate_strike_price(7_000_000).is_ok());
    assert!(validate_strike_price(1_000_000_00).is_ok());
    assert!(validate_strike_price(2_000_000_00).is_err());

    // Test quantity boundaries
    assert!(validate_quantity(0).is_err());
    assert!(validate_quantity(5_000).is_err()); // Too small
    assert!(validate_quantity(10_000).is_ok()); // Min 0.0001 BTC
    assert!(validate_quantity(100_000_000).is_ok()); // Max 1 BTC
    assert!(validate_quantity(200_000_000).is_err()); // Too large
}

fn validate_strike_price(strike: u64) -> Result<(), &'static str> {
    if strike == 0 {
        return Err("Strike price must be greater than 0");
    }
    if strike > 1_000_000_00 {
        return Err("Strike price too high");
    }
    Ok(())
}

fn validate_quantity(quantity: u64) -> Result<(), &'static str> {
    if quantity == 0 {
        return Err("Quantity must be greater than 0");
    }
    if quantity < 10_000 {
        return Err("Quantity too small");
    }
    if quantity > 100_000_000 {
        return Err("Quantity too large");
    }
    Ok(())
}

#[test]
fn test_utilization_rate() {
    // Given
    let pool = SimplePoolState {
        total_liquidity: 100_000_000,
        locked_collateral: 30_000_000,
        available_liquidity: 70_000_000,
        total_premium_collected: 5_000_000,
        total_payout: 2_000_000,
        active_options: 3,
    };

    // When
    let utilization = (pool.locked_collateral as f64 / pool.total_liquidity as f64) * 100.0;

    // Then
    assert_eq!(utilization, 30.0);
}

#[test]
fn test_profit_loss_calculation() {
    // Given - Option that went ITM
    let premium_paid = 500_000;   // 0.005 BTC
    let payout_amount = 666_666;  // 0.00667 BTC

    // When
    let profit_loss = payout_amount as i64 - premium_paid as i64;

    // Then
    assert_eq!(profit_loss, 166_666); // Profit of 0.00167 BTC
}

#[test]
fn test_multiple_option_types() {
    let options = vec![
        SimpleOption {
            option_id: "CALL-001".to_string(),
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 10_000_000,
            premium_paid: 100_000,
            expiry_height: 801_000,
            status: OptionStatus::Active,
            user_id: "user1".to_string(),
        },
        SimpleOption {
            option_id: "PUT-001".to_string(),
            option_type: OptionType::Put,
            strike_price: 7_000_000,
            quantity: 20_000_000,
            premium_paid: 200_000,
            expiry_height: 801_000,
            status: OptionStatus::Active,
            user_id: "user2".to_string(),
        },
    ];

    // Count by type
    let call_count = options.iter().filter(|o| o.option_type == OptionType::Call).count();
    let put_count = options.iter().filter(|o| o.option_type == OptionType::Put).count();

    assert_eq!(call_count, 1);
    assert_eq!(put_count, 1);
    assert_eq!(options.len(), 2);
}
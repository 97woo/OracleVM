// Simple Contract Unit Tests
// These tests verify the SimpleContractManager functionality

use btcfi_contracts::{SimpleContractManager, OptionType, OptionStatus};

#[test]
fn test_manager_creation() {
    let manager = SimpleContractManager::new();
    assert_eq!(manager.options.len(), 0);
    assert_eq!(manager.pool_state.total_liquidity, 0);
}

#[test]
fn test_add_liquidity() {
    let mut manager = SimpleContractManager::new();
    
    // Add 1 BTC liquidity
    let result = manager.add_liquidity(100_000_000);
    assert!(result.is_ok());
    assert_eq!(manager.pool_state.total_liquidity, 100_000_000);
    assert_eq!(manager.pool_state.available_liquidity, 100_000_000);
}

#[test]
fn test_create_call_option() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(100_000_000).unwrap();
    
    let result = manager.create_option(
        "CALL-001".to_string(),
        OptionType::Call,
        70_000_00,    // $70,000
        10_000_000,   // 0.1 BTC
        250_000,      // 0.0025 BTC premium
        800_000,
        "user1".to_string()
    );
    
    assert!(result.is_ok());
    assert_eq!(manager.options.len(), 1);
    assert_eq!(manager.pool_state.active_options, 1);
    assert_eq!(manager.pool_state.locked_collateral, 10_000_000);
}

#[test]
fn test_create_put_option() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(100_000_000).unwrap();
    
    let result = manager.create_option(
        "PUT-001".to_string(),
        OptionType::Put,
        70_000_00,
        10_000_000,
        300_000,
        800_000,
        "user2".to_string()
    );
    
    assert!(result.is_ok());
    let expected_collateral = (70_000_00_u64 * 10_000_000) / 100_000_000;
    assert_eq!(manager.pool_state.locked_collateral, expected_collateral);
}

#[test]
fn test_insufficient_liquidity() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(5_000_000).unwrap(); // Only 0.05 BTC
    
    let result = manager.create_option(
        "CALL-001".to_string(),
        OptionType::Call,
        70_000_00,
        10_000_000,   // Needs 0.1 BTC
        250_000,
        800_000,
        "user1".to_string()
    );
    
    assert!(result.is_err());
    assert_eq!(manager.options.len(), 0);
}

#[test]
fn test_settle_call_itm() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(100_000_000).unwrap();
    
    manager.create_option(
        "CALL-001".to_string(),
        OptionType::Call,
        70_000_00,
        10_000_000,
        250_000,
        800_000,
        "user1".to_string()
    ).unwrap();
    
    // Settle at $75,000 (ITM)
    let payout = manager.settle_option("CALL-001", 75_000_00).unwrap();
    
    assert!(payout > 0);
    let option = manager.options.get("CALL-001").unwrap();
    assert_eq!(option.status, OptionStatus::Settled);
    assert_eq!(manager.pool_state.active_options, 0);
}

#[test]
fn test_settle_call_otm() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(100_000_000).unwrap();
    
    manager.create_option(
        "CALL-001".to_string(),
        OptionType::Call,
        70_000_00,
        10_000_000,
        250_000,
        800_000,
        "user1".to_string()
    ).unwrap();
    
    println!("After create_option - Available: {}, Locked: {}, Total: {}", 
        manager.pool_state.available_liquidity,
        manager.pool_state.locked_collateral,
        manager.pool_state.total_liquidity);
    
    // Settle at $65,000 (OTM)
    let payout = manager.settle_option("CALL-001", 65_000_00).unwrap();
    
    assert_eq!(payout, 0);
    println!("After settlement - Available: {}, Locked: {}, Total: {}", 
        manager.pool_state.available_liquidity,
        manager.pool_state.locked_collateral,
        manager.pool_state.total_liquidity);
    assert_eq!(manager.pool_state.available_liquidity, 100_250_000);
}

#[test]
fn test_settle_put_itm() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(100_000_000).unwrap();
    
    manager.create_option(
        "PUT-001".to_string(),
        OptionType::Put,
        70_000_00,
        10_000_000,
        300_000,
        800_000,
        "user2".to_string()
    ).unwrap();
    
    // Settle at $65,000 (ITM)
    let payout = manager.settle_option("PUT-001", 65_000_00).unwrap();
    
    assert!(payout > 0);
    assert_eq!(manager.pool_state.active_options, 0);
}

#[test]
fn test_get_expired_options() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(200_000_000).unwrap();
    
    // Create options with different expiries
    manager.create_option(
        "OPT-1".to_string(),
        OptionType::Call,
        70_000_00,
        10_000_000,
        100_000,
        800_000,
        "user1".to_string()
    ).unwrap();
    
    manager.create_option(
        "OPT-2".to_string(),
        OptionType::Put,
        70_000_00,
        10_000_000,
        100_000,
        799_000,
        "user2".to_string()
    ).unwrap();
    
    manager.create_option(
        "OPT-3".to_string(),
        OptionType::Call,
        70_000_00,
        10_000_000,
        100_000,
        801_000,
        "user3".to_string()
    ).unwrap();
    
    let expired = manager.get_expired_options(800_000);
    assert_eq!(expired.len(), 2);
}

#[test]
fn test_system_status() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(100_000_000).unwrap();
    
    manager.create_option(
        "CALL-001".to_string(),
        OptionType::Call,
        70_000_00,
        10_000_000,
        250_000,
        800_000,
        "user1".to_string()
    ).unwrap();
    
    let status = manager.get_system_status();
    
    assert!(status["pool_state"].is_object());
    assert_eq!(status["total_options"], 1);
    assert_eq!(status["active_options"], 1);
    assert_eq!(status["profit_loss"], 250_000);
}

#[test]
fn test_utilization_rate() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(100_000_000).unwrap();
    
    manager.create_option(
        "CALL-001".to_string(),
        OptionType::Call,
        70_000_00,
        30_000_000, // 0.3 BTC
        1_000_000,
        800_000,
        "user1".to_string()
    ).unwrap();
    
    let utilization = manager.pool_state.utilization_rate();
    // 프리미엄 1M이 추가되어 total_liquidity는 101M
    // 30M / 101M = 29.7%
    let expected_utilization = 30_000_000.0 / 101_000_000.0 * 100.0;
    assert!((utilization - expected_utilization).abs() < 0.01);
}

#[test]
fn test_premium_collection() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(100_000_000).unwrap();
    
    // Create multiple options to collect premiums
    manager.create_option(
        "CALL-001".to_string(),
        OptionType::Call,
        70_000_00,
        10_000_000,
        500_000, // 0.005 BTC premium
        800_000,
        "user1".to_string()
    ).unwrap();
    
    manager.create_option(
        "PUT-001".to_string(),
        OptionType::Put,
        70_000_00,
        10_000_000,
        300_000, // 0.003 BTC premium
        800_000,
        "user2".to_string()
    ).unwrap();
    
    assert_eq!(manager.pool_state.total_premium_collected, 800_000);
    assert_eq!(manager.pool_state.total_liquidity, 100_800_000);
}

#[test]
fn test_profit_after_settlements() {
    let mut manager = SimpleContractManager::new();
    manager.add_liquidity(200_000_000).unwrap();
    
    // Create options
    manager.create_option(
        "CALL-001".to_string(),
        OptionType::Call,
        70_000_00,
        10_000_000,
        500_000,
        800_000,
        "user1".to_string()
    ).unwrap();
    
    manager.create_option(
        "PUT-001".to_string(),
        OptionType::Put,
        70_000_00,
        10_000_000,
        300_000,
        800_000,
        "user2".to_string()
    ).unwrap();
    
    // Settle Call ITM, Put OTM
    let call_payout = manager.settle_option("CALL-001", 75_000_00).unwrap();
    let _put_payout = manager.settle_option("PUT-001", 75_000_00).unwrap();
    
    let status = manager.get_system_status();
    let profit_loss = status["profit_loss"].as_i64().unwrap();
    assert_eq!(profit_loss, 800_000 - call_payout as i64);
}
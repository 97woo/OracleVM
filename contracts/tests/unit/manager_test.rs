use btcfi_contracts::{SimpleContractManager, OptionType, OptionStatus};

#[cfg(test)]
mod contract_manager_creation {
    use super::*;

    #[test]
    fn test_new_manager() {
        // When
        let manager = SimpleContractManager::new();

        // Then
        assert_eq!(manager.options.len(), 0);
        assert_eq!(manager.pool_state.total_liquidity, 0);
        assert_eq!(manager.pool_state.available_liquidity, 0);
    }

    #[test]
    fn test_initial_pool_state() {
        // Given
        let manager = SimpleContractManager::new();

        // Then
        assert_eq!(manager.pool_state.locked_collateral, 0);
        assert_eq!(manager.pool_state.total_premium_collected, 0);
        assert_eq!(manager.pool_state.total_payout, 0);
        assert_eq!(manager.pool_state.active_options, 0);
        assert_eq!(manager.pool_state.utilization_rate(), 0.0);
    }
}

#[cfg(test)]
mod liquidity_management {
    use super::*;

    #[test]
    fn test_add_liquidity() {
        // Given
        let mut manager = SimpleContractManager::new();

        // When
        let result = manager.add_liquidity(100_000_000); // 1 BTC

        // Then
        assert!(result.is_ok());
        assert_eq!(manager.pool_state.total_liquidity, 100_000_000);
        assert_eq!(manager.pool_state.available_liquidity, 100_000_000);
    }

    #[test]
    fn test_multiple_liquidity_additions() {
        // Given
        let mut manager = SimpleContractManager::new();

        // When
        manager.add_liquidity(50_000_000).unwrap();  // 0.5 BTC
        manager.add_liquidity(30_000_000).unwrap();  // 0.3 BTC
        manager.add_liquidity(20_000_000).unwrap();  // 0.2 BTC

        // Then
        assert_eq!(manager.pool_state.total_liquidity, 100_000_000);
        assert_eq!(manager.pool_state.available_liquidity, 100_000_000);
    }
}

#[cfg(test)]
mod option_creation {
    use super::*;

    #[test]
    fn test_create_call_option_success() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(100_000_000).unwrap();

        // When
        let result = manager.create_option(
            "CALL-001".to_string(),
            OptionType::Call,
            70_000_00,    // $70,000 strike
            10_000_000,   // 0.1 BTC quantity
            250_000,      // 0.0025 BTC premium
            800_000,      // expiry height
            "user1".to_string()
        );

        // Then
        assert!(result.is_ok());
        assert_eq!(manager.options.len(), 1);
        assert_eq!(manager.pool_state.active_options, 1);
        assert_eq!(manager.pool_state.locked_collateral, 10_000_000);
        assert_eq!(manager.pool_state.available_liquidity, 90_250_000); // 100M - 10M + 0.25M
        assert_eq!(manager.pool_state.total_premium_collected, 250_000);
    }

    #[test]
    fn test_create_put_option_success() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(100_000_000).unwrap();

        // When
        let result = manager.create_option(
            "PUT-001".to_string(),
            OptionType::Put,
            70_000_00,    // $70,000 strike
            10_000_000,   // 0.1 BTC quantity
            300_000,      // 0.003 BTC premium
            800_000,
            "user2".to_string()
        );

        // Then
        assert!(result.is_ok());
        
        // Put option collateral = (strike * quantity) / 100_000_000
        let expected_collateral = (70_000_00_u64 * 10_000_000) / 100_000_000;
        assert_eq!(manager.pool_state.locked_collateral, expected_collateral);
    }

    #[test]
    fn test_create_option_insufficient_liquidity() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(5_000_000).unwrap(); // Only 0.05 BTC

        // When
        let result = manager.create_option(
            "CALL-001".to_string(),
            OptionType::Call,
            70_000_00,
            10_000_000,   // 0.1 BTC needed
            250_000,
            800_000,
            "user1".to_string()
        );

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient liquidity"));
        assert_eq!(manager.options.len(), 0);
        assert_eq!(manager.pool_state.active_options, 0);
    }

    #[test]
    fn test_multiple_options_creation() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(200_000_000).unwrap(); // 2 BTC

        // When - Create multiple options
        let call1 = manager.create_option(
            "CALL-001".to_string(),
            OptionType::Call,
            70_000_00,
            10_000_000,
            250_000,
            800_000,
            "user1".to_string()
        );

        let put1 = manager.create_option(
            "PUT-001".to_string(),
            OptionType::Put,
            65_000_00,
            5_000_000,
            150_000,
            800_000,
            "user2".to_string()
        );

        let call2 = manager.create_option(
            "CALL-002".to_string(),
            OptionType::Call,
            75_000_00,
            20_000_000,
            500_000,
            801_000,
            "user3".to_string()
        );

        // Then
        assert!(call1.is_ok());
        assert!(put1.is_ok());
        assert!(call2.is_ok());
        assert_eq!(manager.options.len(), 3);
        assert_eq!(manager.pool_state.active_options, 3);
        assert_eq!(manager.pool_state.total_premium_collected, 900_000); // 250k + 150k + 500k
    }
}

#[cfg(test)]
mod option_settlement {
    use super::*;

    #[test]
    fn test_settle_call_option_itm() {
        // Given
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

        // When - Spot price $75,000 (ITM)
        let payout = manager.settle_option("CALL-001", 75_000_00).unwrap();

        // Then
        assert!(payout > 0);
        let option = manager.options.get("CALL-001").unwrap();
        assert_eq!(option.status, OptionStatus::Settled);
        assert_eq!(manager.pool_state.active_options, 0);
        assert_eq!(manager.pool_state.total_payout, payout);
    }

    #[test]
    fn test_settle_call_option_otm() {
        // Given
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

        // When - Spot price $65,000 (OTM)
        let payout = manager.settle_option("CALL-001", 65_000_00).unwrap();

        // Then
        assert_eq!(payout, 0);
        assert_eq!(manager.pool_state.active_options, 0);
        assert_eq!(manager.pool_state.total_payout, 0);
        // Collateral should be returned to available liquidity
        assert_eq!(manager.pool_state.available_liquidity, 100_250_000);
    }

    #[test]
    fn test_settle_put_option_itm() {
        // Given
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

        // When - Spot price $65,000 (ITM)
        let payout = manager.settle_option("PUT-001", 65_000_00).unwrap();

        // Then
        assert!(payout > 0);
        assert_eq!(manager.pool_state.active_options, 0);
        assert_eq!(manager.pool_state.total_payout, payout);
    }

    #[test]
    fn test_settle_option_not_found() {
        // Given
        let mut manager = SimpleContractManager::new();

        // When
        let result = manager.settle_option("INVALID-ID", 70_000_00);

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Option not found"));
    }

    #[test]
    fn test_settle_already_settled_option() {
        // Given
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
        
        // First settlement
        manager.settle_option("CALL-001", 75_000_00).unwrap();

        // When - Try to settle again
        let result = manager.settle_option("CALL-001", 75_000_00);

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Option not active"));
    }
}

#[cfg(test)]
mod expired_options_query {
    use super::*;

    #[test]
    fn test_get_expired_options() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(200_000_000).unwrap();
        
        // Create options with different expiry heights
        manager.create_option(
            "OPT-1".to_string(),
            OptionType::Call,
            70_000_00,
            10_000_000,
            100_000,
            800_000, // expires at 800k
            "user1".to_string()
        ).unwrap();
        
        manager.create_option(
            "OPT-2".to_string(),
            OptionType::Put,
            70_000_00,
            10_000_000,
            100_000,
            799_000, // expires at 799k
            "user2".to_string()
        ).unwrap();
        
        manager.create_option(
            "OPT-3".to_string(),
            OptionType::Call,
            70_000_00,
            10_000_000,
            100_000,
            801_000, // expires at 801k
            "user3".to_string()
        ).unwrap();

        // When - Current height is 800_000
        let expired = manager.get_expired_options(800_000);

        // Then - Options 1 and 2 should be expired
        assert_eq!(expired.len(), 2);
        let expired_ids: Vec<&str> = expired.iter().map(|o| o.option_id.as_str()).collect();
        assert!(expired_ids.contains(&"OPT-1"));
        assert!(expired_ids.contains(&"OPT-2"));
        assert!(!expired_ids.contains(&"OPT-3"));
    }

    #[test]
    fn test_no_expired_options() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(100_000_000).unwrap();
        
        manager.create_option(
            "OPT-1".to_string(),
            OptionType::Call,
            70_000_00,
            10_000_000,
            100_000,
            800_000,
            "user1".to_string()
        ).unwrap();

        // When - Current height is before expiry
        let expired = manager.get_expired_options(799_000);

        // Then
        assert_eq!(expired.len(), 0);
    }
}

#[cfg(test)]
mod system_status {
    use super::*;

    #[test]
    fn test_get_system_status() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(100_000_000).unwrap();
        
        // Create and settle some options
        manager.create_option(
            "CALL-001".to_string(),
            OptionType::Call,
            70_000_00,
            10_000_000,
            250_000,
            800_000,
            "user1".to_string()
        ).unwrap();

        // When
        let status = manager.get_system_status();

        // Then
        assert!(status["pool_state"].is_object());
        assert_eq!(status["total_options"], 1);
        assert_eq!(status["active_options"], 1);
        assert!(status["utilization_rate"].is_string());
        assert_eq!(status["profit_loss"], 250_000); // Only premium collected, no payouts
    }

    #[test]
    fn test_system_status_after_settlements() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(200_000_000).unwrap();
        
        // Create and settle multiple options
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

        // Settle Call ITM
        let call_payout = manager.settle_option("CALL-001", 75_000_00).unwrap();
        
        // Settle Put OTM
        let _put_payout = manager.settle_option("PUT-001", 75_000_00).unwrap();

        // When
        let status = manager.get_system_status();

        // Then
        assert_eq!(status["total_options"], 2);
        assert_eq!(status["active_options"], 0); // All settled
        let profit_loss = status["profit_loss"].as_i64().unwrap();
        assert_eq!(profit_loss, 800_000 - call_payout as i64); // Premium - payouts
    }
}

#[cfg(test)]
mod pool_utilization {
    use super::*;

    #[test]
    fn test_utilization_rate_calculation() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(100_000_000).unwrap();

        // When - Create options to lock 30% of liquidity
        manager.create_option(
            "CALL-001".to_string(),
            OptionType::Call,
            70_000_00,
            30_000_000, // 0.3 BTC
            1_000_000,
            800_000,
            "user1".to_string()
        ).unwrap();

        // Then
        let utilization = manager.pool_state.utilization_rate();
        // 프리미엄 1M이 추가되어 total_liquidity는 101M
        // 30M / 101M = 29.7%
        let expected_utilization = 30_000_000.0 / 101_000_000.0 * 100.0;
        assert!((utilization - expected_utilization).abs() < 0.01);
    }

    #[test]
    fn test_utilization_with_mixed_options() {
        // Given
        let mut manager = SimpleContractManager::new();
        manager.add_liquidity(200_000_000).unwrap(); // 2 BTC

        // When - Create call and put options
        manager.create_option(
            "CALL-001".to_string(),
            OptionType::Call,
            70_000_00,
            20_000_000, // 0.2 BTC collateral
            500_000,
            800_000,
            "user1".to_string()
        ).unwrap();

        manager.create_option(
            "PUT-001".to_string(),
            OptionType::Put,
            60_000_00,
            10_000_000, // Collateral = (60k * 0.1) / 100M = 0.06 BTC
            300_000,
            800_000,
            "user2".to_string()
        ).unwrap();

        // Then
        let expected_locked = 20_000_000 + (60_000_00_u64 * 10_000_000) / 100_000_000;
        assert_eq!(manager.pool_state.locked_collateral, expected_locked);
        
        let utilization = manager.pool_state.utilization_rate();
        // total_liquidity는 200M + 500K + 300K = 200.8M
        let expected_util = (expected_locked as f64 / 200_800_000.0) * 100.0;
        assert!((utilization - expected_util).abs() < 0.01);
    }
}
use btcfi_contracts::{SimplePoolState, OptionType};

#[cfg(test)]
mod pool_state {
    use super::*;

    #[test]
    fn test_new_pool_state() {
        // When
        let pool = SimplePoolState::new();

        // Then
        assert_eq!(pool.total_liquidity, 0);
        assert_eq!(pool.locked_collateral, 0);
        assert_eq!(pool.available_liquidity, 0);
        assert_eq!(pool.total_premium_collected, 0);
        assert_eq!(pool.total_payout, 0);
        assert_eq!(pool.active_options, 0);
    }

    #[test]
    fn test_utilization_rate_empty_pool() {
        // Given
        let pool = SimplePoolState::new();

        // When
        let utilization = pool.utilization_rate();

        // Then
        assert_eq!(utilization, 0.0);
    }

    #[test]
    fn test_utilization_rate_with_collateral() {
        // Given
        let pool = SimplePoolState {
            total_liquidity: 100_000_000,      // 1 BTC
            locked_collateral: 30_000_000,     // 0.3 BTC
            available_liquidity: 70_000_000,   // 0.7 BTC
            total_premium_collected: 0,
            total_payout: 0,
            active_options: 3,
        };

        // When
        let utilization = pool.utilization_rate();

        // Then
        assert_eq!(utilization, 30.0); // 30%
    }
}

#[cfg(test)]
mod pool_operations {
    use super::*;

    fn add_liquidity(pool: &mut SimplePoolState, amount: u64) -> Result<(), &'static str> {
        if amount == 0 {
            return Err("Amount must be greater than 0");
        }
        if amount < 100_000 { // Min 0.001 BTC
            return Err("Amount below minimum");
        }
        
        pool.total_liquidity += amount;
        pool.available_liquidity += amount;
        Ok(())
    }

    fn lock_collateral(pool: &mut SimplePoolState, amount: u64) -> Result<(), &'static str> {
        if amount > pool.available_liquidity {
            return Err("Insufficient available liquidity");
        }
        
        pool.locked_collateral += amount;
        pool.available_liquidity -= amount;
        pool.active_options += 1;
        Ok(())
    }

    fn release_collateral(pool: &mut SimplePoolState, amount: u64) -> Result<(), &'static str> {
        if amount > pool.locked_collateral {
            return Err("Amount exceeds locked collateral");
        }
        
        pool.locked_collateral -= amount;
        pool.available_liquidity += amount;
        pool.active_options = pool.active_options.saturating_sub(1);
        Ok(())
    }

    #[test]
    fn test_add_liquidity_success() {
        // Given
        let mut pool = SimplePoolState::new();

        // When
        let result = add_liquidity(&mut pool, 10_000_000); // 0.1 BTC

        // Then
        assert!(result.is_ok());
        assert_eq!(pool.total_liquidity, 10_000_000);
        assert_eq!(pool.available_liquidity, 10_000_000);
    }

    #[test]
    fn test_add_liquidity_zero_amount() {
        // Given
        let mut pool = SimplePoolState::new();

        // When
        let result = add_liquidity(&mut pool, 0);

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Amount must be greater than 0");
    }

    #[test]
    fn test_add_liquidity_below_minimum() {
        // Given
        let mut pool = SimplePoolState::new();

        // When
        let result = add_liquidity(&mut pool, 50_000); // 0.0005 BTC

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Amount below minimum");
    }

    #[test]
    fn test_lock_collateral_success() {
        // Given
        let mut pool = SimplePoolState::new();
        add_liquidity(&mut pool, 100_000_000).unwrap(); // 1 BTC

        // When
        let result = lock_collateral(&mut pool, 30_000_000); // 0.3 BTC

        // Then
        assert!(result.is_ok());
        assert_eq!(pool.locked_collateral, 30_000_000);
        assert_eq!(pool.available_liquidity, 70_000_000);
        assert_eq!(pool.active_options, 1);
    }

    #[test]
    fn test_lock_collateral_insufficient_liquidity() {
        // Given
        let mut pool = SimplePoolState::new();
        add_liquidity(&mut pool, 10_000_000).unwrap(); // 0.1 BTC

        // When
        let result = lock_collateral(&mut pool, 20_000_000); // 0.2 BTC

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Insufficient available liquidity");
    }

    #[test]
    fn test_release_collateral_success() {
        // Given
        let mut pool = SimplePoolState::new();
        add_liquidity(&mut pool, 100_000_000).unwrap();
        lock_collateral(&mut pool, 30_000_000).unwrap();

        // When
        let result = release_collateral(&mut pool, 30_000_000);

        // Then
        assert!(result.is_ok());
        assert_eq!(pool.locked_collateral, 0);
        assert_eq!(pool.available_liquidity, 100_000_000);
        assert_eq!(pool.active_options, 0);
    }

    #[test]
    fn test_premium_collection() {
        // Given
        let mut pool = SimplePoolState::new();
        add_liquidity(&mut pool, 100_000_000).unwrap();

        // When
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
    fn test_settlement_payout() {
        // Given
        let mut pool = SimplePoolState::new();
        add_liquidity(&mut pool, 100_000_000).unwrap();
        lock_collateral(&mut pool, 50_000_000).unwrap();

        // When
        let payout = 20_000_000; // 0.2 BTC
        pool.locked_collateral -= payout;
        pool.total_liquidity -= payout;
        pool.total_payout += payout;

        // Then
        assert_eq!(pool.locked_collateral, 30_000_000);
        assert_eq!(pool.total_liquidity, 80_000_000);
        assert_eq!(pool.total_payout, 20_000_000);
    }
}

#[cfg(test)]
mod pool_calculations {
    use super::*;

    fn calculate_required_collateral(option_type: OptionType, quantity: u64, strike_price: u64) -> u64 {
        match option_type {
            OptionType::Call => quantity,
            OptionType::Put => {
                // Assuming BTC price = $70,000 for simplicity
                (strike_price * quantity) / 70_000_00
            }
        }
    }

    #[test]
    fn test_call_option_collateral() {
        // Given
        let collateral = calculate_required_collateral(
            OptionType::Call,
            50_000_000,  // 0.5 BTC
            70_000_00    // $70,000 strike
        );

        // Then
        assert_eq!(collateral, 50_000_000); // Same as quantity for calls
    }

    #[test]
    fn test_put_option_collateral() {
        // Given
        let collateral = calculate_required_collateral(
            OptionType::Put,
            10_000_000,  // 0.1 BTC
            70_000_00    // $70,000 strike
        );

        // Then
        assert_eq!(collateral, 10_000_000); // Equal to notional at current price
    }

    #[test]
    fn test_pool_profit_loss() {
        // Given
        let initial_liquidity = 100_000_000;
        let premium_collected = 5_000_000;
        let total_payout = 3_000_000;

        // When
        let final_liquidity = initial_liquidity + premium_collected - total_payout;
        let profit = premium_collected as i64 - total_payout as i64;

        // Then
        assert_eq!(final_liquidity, 102_000_000);
        assert_eq!(profit, 2_000_000); // 0.02 BTC profit
    }
}
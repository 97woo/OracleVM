use btcfi_contracts::{OptionType, OptionStatus, SimpleOption};

#[cfg(test)]
mod option_creation {
    use super::*;

    #[test]
    fn test_create_call_option() {
        // Given
        let option = SimpleOption {
            option_id: "OPT-001".to_string(),
            option_type: OptionType::Call,
            strike_price: 70_000_00, // $70,000 in cents
            quantity: 10_000_000,    // 0.1 BTC
            premium_paid: 100_000,   // 0.001 BTC
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user123".to_string(),
        };

        // Then
        assert_eq!(option.option_type, OptionType::Call);
        assert_eq!(option.strike_price, 70_000_00);
        assert_eq!(option.status, OptionStatus::Active);
    }

    #[test]
    fn test_create_put_option() {
        // Given
        let option = SimpleOption {
            option_id: "OPT-002".to_string(),
            option_type: OptionType::Put,
            strike_price: 65_000_00,
            quantity: 50_000_000,
            premium_paid: 200_000,
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user456".to_string(),
        };

        // Then
        assert_eq!(option.option_type, OptionType::Put);
        assert_eq!(option.quantity, 50_000_000);
    }
}

#[cfg(test)]
mod option_validation {

    fn validate_strike_price(strike: u64) -> Result<(), &'static str> {
        if strike == 0 {
            return Err("Strike price must be greater than 0");
        }
        if strike > 1_000_000_00 { // $1M limit
            return Err("Strike price exceeds maximum");
        }
        Ok(())
    }

    fn validate_quantity(quantity: u64) -> Result<(), &'static str> {
        if quantity == 0 {
            return Err("Quantity must be greater than 0");
        }
        if quantity < 10_000 { // Min 0.0001 BTC
            return Err("Quantity below minimum");
        }
        if quantity > 100_000_000 { // Max 1 BTC
            return Err("Quantity exceeds maximum");
        }
        Ok(())
    }

    #[test]
    fn test_validate_strike_price() {
        assert!(validate_strike_price(0).is_err());
        assert!(validate_strike_price(70_000_00).is_ok());
        assert!(validate_strike_price(1_000_000_00).is_ok());
        assert!(validate_strike_price(2_000_000_00).is_err());
    }

    #[test]
    fn test_validate_quantity() {
        assert!(validate_quantity(0).is_err());
        assert!(validate_quantity(5_000).is_err());
        assert!(validate_quantity(10_000).is_ok());
        assert!(validate_quantity(100_000_000).is_ok());
        assert!(validate_quantity(200_000_000).is_err());
    }
}

#[cfg(test)]
mod option_settlement {
    use super::*;

    fn is_in_the_money(option: &SimpleOption, spot_price: u64) -> bool {
        match option.option_type {
            OptionType::Call => spot_price > option.strike_price,
            OptionType::Put => spot_price < option.strike_price,
        }
    }

    fn calculate_payout(option: &SimpleOption, spot_price: u64) -> u64 {
        if !is_in_the_money(option, spot_price) {
            return 0;
        }

        match option.option_type {
            OptionType::Call => {
                let price_diff = spot_price - option.strike_price;
                (price_diff as u128 * option.quantity as u128 / spot_price as u128) as u64
            }
            OptionType::Put => {
                let price_diff = option.strike_price - spot_price;
                (price_diff as u128 * option.quantity as u128 / spot_price as u128) as u64
            }
        }
    }

    #[test]
    fn test_call_option_itm() {
        // Given
        let option = SimpleOption {
            option_id: "CALL-001".to_string(),
            option_type: OptionType::Call,
            strike_price: 70_000_00,
            quantity: 10_000_000,
            premium_paid: 100_000,
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user123".to_string(),
        };
        let spot_price = 75_000_00; // $75,000

        // When
        let is_itm = is_in_the_money(&option, spot_price);
        let payout = calculate_payout(&option, spot_price);

        // Then
        assert!(is_itm);
        assert_eq!(payout, 666_666); // ~0.00667 BTC
    }

    #[test]
    fn test_call_option_otm() {
        // Given
        let option = SimpleOption {
            option_id: "CALL-002".to_string(),
            option_type: OptionType::Call,
            strike_price: 70_000_00,
            quantity: 10_000_000,
            premium_paid: 100_000,
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user123".to_string(),
        };
        let spot_price = 65_000_00;

        // When
        let is_itm = is_in_the_money(&option, spot_price);
        let payout = calculate_payout(&option, spot_price);

        // Then
        assert!(!is_itm);
        assert_eq!(payout, 0);
    }

    #[test]
    fn test_put_option_itm() {
        // Given
        let option = SimpleOption {
            option_id: "PUT-001".to_string(),
            option_type: OptionType::Put,
            strike_price: 70_000_00,
            quantity: 10_000_000,
            premium_paid: 100_000,
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user123".to_string(),
        };
        let spot_price = 65_000_00;

        // When
        let is_itm = is_in_the_money(&option, spot_price);
        let payout = calculate_payout(&option, spot_price);

        // Then
        assert!(is_itm);
        assert_eq!(payout, 769_230); // ~0.00769 BTC
    }

    #[test]
    fn test_put_option_otm() {
        // Given
        let option = SimpleOption {
            option_id: "PUT-002".to_string(),
            option_type: OptionType::Put,
            strike_price: 70_000_00,
            quantity: 10_000_000,
            premium_paid: 100_000,
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user123".to_string(),
        };
        let spot_price = 75_000_00;

        // When
        let is_itm = is_in_the_money(&option, spot_price);
        let payout = calculate_payout(&option, spot_price);

        // Then
        assert!(!is_itm);
        assert_eq!(payout, 0);
    }

    #[test]
    fn test_option_at_the_money() {
        // Given - Options at the money
        let call = SimpleOption {
            option_id: "CALL-ATM".to_string(),
            option_type: OptionType::Call,
            strike_price: 70_000_00,
            quantity: 10_000_000,
            premium_paid: 100_000,
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user123".to_string(),
        };
        let put = SimpleOption {
            option_id: "PUT-ATM".to_string(),
            option_type: OptionType::Put,
            strike_price: 70_000_00,
            quantity: 10_000_000,
            premium_paid: 100_000,
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user123".to_string(),
        };
        let spot_price = 70_000_00;

        // When & Then
        assert!(!is_in_the_money(&call, spot_price));
        assert!(!is_in_the_money(&put, spot_price));
        assert_eq!(calculate_payout(&call, spot_price), 0);
        assert_eq!(calculate_payout(&put, spot_price), 0);
    }
}
use anyhow::Result;
use btcfi_contracts::{OptionType, OptionStatus, SimpleOption};

/// 옵션 생성 파라미터
#[derive(Debug, Clone)]
pub struct CreateOptionParams {
    pub option_type: OptionType,
    pub strike_price: u64,      // USD cents
    pub quantity: u64,          // satoshis
    pub premium: u64,           // satoshis per unit
    pub expiry_height: u32,
    pub user_id: String,
}

/// 옵션 생성 검증
pub fn validate_option_params(params: &CreateOptionParams) -> Result<()> {
    // 행사가 검증
    if params.strike_price == 0 {
        anyhow::bail!("Strike price must be greater than 0");
    }
    
    if params.strike_price > 1_000_000_00 { // $1M in cents
        anyhow::bail!("Strike price too high");
    }

    // 수량 검증
    if params.quantity == 0 {
        anyhow::bail!("Quantity must be greater than 0");
    }
    
    if params.quantity < 10_000 { // 최소 0.0001 BTC
        anyhow::bail!("Quantity too small (minimum 0.0001 BTC)");
    }
    
    if params.quantity > 100_000_000 { // 최대 1 BTC
        anyhow::bail!("Quantity too large (maximum 1 BTC)");
    }

    // 프리미엄 검증
    if params.premium == 0 {
        anyhow::bail!("Premium must be greater than 0");
    }
    
    // 프리미엄이 행사가의 50%를 초과할 수 없음
    let max_premium = params.strike_price * params.quantity / 200; // 50% of strike * quantity
    if params.premium > max_premium {
        anyhow::bail!("Premium too high (maximum 50% of strike price)");
    }

    // 만기 검증
    let current_height = 800_000; // 현재 블록 높이 (시뮬레이션)
    if params.expiry_height <= current_height {
        anyhow::bail!("Expiry height must be in the future");
    }
    
    if params.expiry_height > current_height + 52_560 { // 최대 1년 (약 52,560 블록)
        anyhow::bail!("Expiry too far in the future (maximum 1 year)");
    }

    // 사용자 ID 검증
    if params.user_id.is_empty() {
        anyhow::bail!("User ID cannot be empty");
    }

    Ok(())
}

/// 필요한 담보 계산
pub fn calculate_required_collateral(params: &CreateOptionParams) -> u64 {
    match params.option_type {
        OptionType::Call => {
            // Call 옵션: 수량만큼의 BTC가 담보로 필요
            params.quantity
        }
        OptionType::Put => {
            // Put 옵션: 행사가 * 수량 / BTC 가격이 담보로 필요
            // 간단히 하기 위해 BTC = $70,000로 가정
            let btc_price_cents = 7_000_000; // $70,000 in cents
            (params.strike_price * params.quantity) / btc_price_cents
        }
    }
}

/// 옵션 생성
pub fn create_option(params: CreateOptionParams, option_id: String) -> Result<SimpleOption> {
    // 파라미터 검증
    validate_option_params(&params)?;
    
    let premium_paid = params.premium * (params.quantity / 100_000_000); // 프리미엄 총액
    
    Ok(SimpleOption {
        option_id,
        option_type: params.option_type,
        strike_price: params.strike_price,
        quantity: params.quantity,
        premium_paid,
        expiry_height: params.expiry_height,
        status: OptionStatus::Active,
        user_id: params.user_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_call_option() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000, // $70,000 in cents
            quantity: 10_000_000,     // 0.1 BTC
            premium: 100_000,         // 0.001 BTC premium
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let option = create_option(params, "OPT-001".to_string()).unwrap();

        // Then
        assert_eq!(option.option_id, "OPT-001");
        assert_eq!(option.option_type, OptionType::Call);
        assert_eq!(option.strike_price, 7_000_000);
        assert_eq!(option.quantity, 10_000_000);
        assert_eq!(option.status, OptionStatus::Active);
    }

    #[test]
    fn test_create_valid_put_option() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Put,
            strike_price: 6_500_000, // $65,000 in cents
            quantity: 50_000_000,     // 0.5 BTC
            premium: 200_000,         // 0.002 BTC premium
            expiry_height: 802_000,
            user_id: "user456".to_string(),
        };

        // When
        let option = create_option(params, "OPT-002".to_string()).unwrap();

        // Then
        assert_eq!(option.option_type, OptionType::Put);
        assert_eq!(option.strike_price, 6_500_000);
    }

    #[test]
    fn test_reject_zero_strike_price() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 0,
            quantity: 10_000_000,
            premium: 100_000,
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Strike price must be greater than 0");
    }

    #[test]
    fn test_reject_excessive_strike_price() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 2_000_000_00, // $2M in cents
            quantity: 10_000_000,
            premium: 100_000,
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Strike price too high");
    }

    #[test]
    fn test_reject_zero_quantity() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 0,
            premium: 100_000,
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Quantity must be greater than 0");
    }

    #[test]
    fn test_reject_quantity_too_small() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 5_000, // Less than 0.0001 BTC
            premium: 100_000,
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Quantity too small (minimum 0.0001 BTC)");
    }

    #[test]
    fn test_reject_quantity_too_large() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 200_000_000, // More than 1 BTC
            premium: 100_000,
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Quantity too large (maximum 1 BTC)");
    }

    #[test]
    fn test_reject_expired_option() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 10_000_000,
            premium: 100_000,
            expiry_height: 799_000, // In the past
            user_id: "user123".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Expiry height must be in the future");
    }

    #[test]
    fn test_reject_expiry_too_far() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 10_000_000,
            premium: 100_000,
            expiry_height: 900_000, // More than 1 year in future
            user_id: "user123".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Expiry too far in the future (maximum 1 year)");
    }

    #[test]
    fn test_reject_excessive_premium() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 10_000_000,
            premium: 40_000_000, // More than 50% of strike
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Premium too high (maximum 50% of strike price)");
    }

    #[test]
    fn test_calculate_call_option_collateral() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 50_000_000, // 0.5 BTC
            premium: 100_000,
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let collateral = calculate_required_collateral(&params);

        // Then
        assert_eq!(collateral, 50_000_000); // Same as quantity for call options
    }

    #[test]
    fn test_calculate_put_option_collateral() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Put,
            strike_price: 7_000_000, // $70,000
            quantity: 10_000_000,    // 0.1 BTC
            premium: 100_000,
            expiry_height: 801_000,
            user_id: "user123".to_string(),
        };

        // When
        let collateral = calculate_required_collateral(&params);

        // Then
        // ($70,000 * 0.1 BTC) / $70,000 per BTC = 0.1 BTC = 10,000,000 sats
        assert_eq!(collateral, 10_000_000);
    }

    #[test]
    fn test_reject_empty_user_id() {
        // Given
        let params = CreateOptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000,
            quantity: 10_000_000,
            premium: 100_000,
            expiry_height: 801_000,
            user_id: "".to_string(),
        };

        // When
        let result = create_option(params, "OPT-001".to_string());

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "User ID cannot be empty");
    }
}
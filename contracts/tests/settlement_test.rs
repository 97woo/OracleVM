use anyhow::Result;
use btcfi_contracts::{OptionType, OptionStatus, SimpleOption};

/// 정산 결과
#[derive(Debug, Clone, PartialEq)]
pub struct SettlementResult {
    pub option_id: String,
    pub is_itm: bool,           // In The Money
    pub payout_amount: u64,     // satoshis
    pub profit_loss: i64,       // satoshis (can be negative)
    pub settlement_type: SettlementType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettlementType {
    Cash,       // 현금 정산
    Physical,   // 실물 인도 (BTC)
}

/// 옵션이 ITM인지 확인
pub fn is_in_the_money(option: &SimpleOption, spot_price: u64) -> bool {
    match option.option_type {
        OptionType::Call => spot_price > option.strike_price,
        OptionType::Put => spot_price < option.strike_price,
    }
}

/// 정산 금액 계산
pub fn calculate_settlement_amount(option: &SimpleOption, spot_price: u64) -> u64 {
    if !is_in_the_money(option, spot_price) {
        return 0;
    }

    let btc_price_cents = spot_price; // spot price is already in cents
    
    match option.option_type {
        OptionType::Call => {
            // (Spot - Strike) * Quantity / BTC_Price
            let price_diff = spot_price.saturating_sub(option.strike_price);
            (price_diff * option.quantity) / btc_price_cents
        }
        OptionType::Put => {
            // (Strike - Spot) * Quantity / BTC_Price
            let price_diff = option.strike_price.saturating_sub(spot_price);
            (price_diff * option.quantity) / btc_price_cents
        }
    }
}

/// 옵션 정산 실행
pub fn settle_option(
    option: &mut SimpleOption,
    spot_price: u64,
    current_height: u32,
) -> Result<SettlementResult> {
    // 상태 확인
    if option.status != OptionStatus::Active {
        anyhow::bail!("Option is not active");
    }

    // 만기 확인
    if current_height < option.expiry_height {
        anyhow::bail!("Option has not expired yet");
    }

    // ITM 여부 확인
    let is_itm = is_in_the_money(option, spot_price);
    let payout_amount = calculate_settlement_amount(option, spot_price);
    
    // 손익 계산 (payout - premium)
    let profit_loss = payout_amount as i64 - option.premium_paid as i64;

    // 상태 업데이트
    option.status = OptionStatus::Settled;

    Ok(SettlementResult {
        option_id: option.option_id.clone(),
        is_itm,
        payout_amount,
        profit_loss,
        settlement_type: SettlementType::Cash,
    })
}

/// 일괄 정산
pub fn batch_settle_options(
    options: &mut [SimpleOption],
    spot_price: u64,
    current_height: u32,
) -> Vec<Result<SettlementResult>> {
    options
        .iter_mut()
        .map(|option| settle_option(option, spot_price, current_height))
        .collect()
}

/// 정산 검증
pub fn validate_settlement(
    option: &SimpleOption,
    spot_price: u64,
    payout: u64,
) -> Result<()> {
    let expected_payout = calculate_settlement_amount(option, spot_price);
    
    if payout != expected_payout {
        anyhow::bail!(
            "Settlement payout mismatch: expected {}, got {}",
            expected_payout,
            payout
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_option(option_type: OptionType, strike: u64) -> SimpleOption {
        SimpleOption {
            option_id: "TEST-001".to_string(),
            option_type,
            strike_price: strike,
            quantity: 10_000_000, // 0.1 BTC
            premium_paid: 100_000, // 0.001 BTC
            expiry_height: 800_000,
            status: OptionStatus::Active,
            user_id: "user123".to_string(),
        }
    }

    #[test]
    fn test_call_option_itm() {
        // Given - Call option with strike $70k, spot $75k
        let option = create_test_option(OptionType::Call, 7_000_000);
        let spot_price = 7_500_000; // $75,000

        // When
        let is_itm = is_in_the_money(&option, spot_price);
        let payout = calculate_settlement_amount(&option, spot_price);

        // Then
        assert!(is_itm);
        // ($75k - $70k) * 0.1 BTC / $75k = 0.00667 BTC = 666,666 sats
        assert_eq!(payout, 666_666);
    }

    #[test]
    fn test_call_option_otm() {
        // Given - Call option with strike $70k, spot $65k
        let option = create_test_option(OptionType::Call, 7_000_000);
        let spot_price = 6_500_000; // $65,000

        // When
        let is_itm = is_in_the_money(&option, spot_price);
        let payout = calculate_settlement_amount(&option, spot_price);

        // Then
        assert!(!is_itm);
        assert_eq!(payout, 0);
    }

    #[test]
    fn test_put_option_itm() {
        // Given - Put option with strike $70k, spot $65k
        let option = create_test_option(OptionType::Put, 7_000_000);
        let spot_price = 6_500_000; // $65,000

        // When
        let is_itm = is_in_the_money(&option, spot_price);
        let payout = calculate_settlement_amount(&option, spot_price);

        // Then
        assert!(is_itm);
        // ($70k - $65k) * 0.1 BTC / $65k = 0.00769 BTC = 769,230 sats
        assert_eq!(payout, 769_230);
    }

    #[test]
    fn test_put_option_otm() {
        // Given - Put option with strike $70k, spot $75k
        let option = create_test_option(OptionType::Put, 7_000_000);
        let spot_price = 7_500_000; // $75,000

        // When
        let is_itm = is_in_the_money(&option, spot_price);
        let payout = calculate_settlement_amount(&option, spot_price);

        // Then
        assert!(!is_itm);
        assert_eq!(payout, 0);
    }

    #[test]
    fn test_option_at_the_money() {
        // Given - Options with strike equal to spot
        let call = create_test_option(OptionType::Call, 7_000_000);
        let put = create_test_option(OptionType::Put, 7_000_000);
        let spot_price = 7_000_000; // $70,000

        // When
        let call_itm = is_in_the_money(&call, spot_price);
        let put_itm = is_in_the_money(&put, spot_price);

        // Then - ATM options are OTM
        assert!(!call_itm);
        assert!(!put_itm);
    }

    #[test]
    fn test_settle_expired_option() {
        // Given
        let mut option = create_test_option(OptionType::Call, 7_000_000);
        let spot_price = 7_500_000;
        let current_height = 800_001; // After expiry

        // When
        let result = settle_option(&mut option, spot_price, current_height).unwrap();

        // Then
        assert_eq!(option.status, OptionStatus::Settled);
        assert!(result.is_itm);
        assert_eq!(result.payout_amount, 666_666);
        assert_eq!(result.profit_loss, 666_666 - 100_000); // payout - premium
    }

    #[test]
    fn test_reject_settle_not_expired() {
        // Given
        let mut option = create_test_option(OptionType::Call, 7_000_000);
        let spot_price = 7_500_000;
        let current_height = 799_999; // Before expiry

        // When
        let result = settle_option(&mut option, spot_price, current_height);

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Option has not expired yet");
    }

    #[test]
    fn test_reject_settle_already_settled() {
        // Given
        let mut option = create_test_option(OptionType::Call, 7_000_000);
        option.status = OptionStatus::Settled;
        let spot_price = 7_500_000;
        let current_height = 800_001;

        // When
        let result = settle_option(&mut option, spot_price, current_height);

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Option is not active");
    }

    #[test]
    fn test_batch_settlement() {
        // Given - Multiple options with different strikes
        let mut options = vec![
            create_test_option(OptionType::Call, 7_000_000), // ITM
            create_test_option(OptionType::Call, 8_000_000), // OTM
            create_test_option(OptionType::Put, 7_000_000),  // OTM
            create_test_option(OptionType::Put, 8_000_000),  // ITM
        ];
        let spot_price = 7_500_000;
        let current_height = 800_001;

        // When
        let results = batch_settle_options(&mut options, spot_price, current_height);

        // Then
        assert_eq!(results.len(), 4);
        assert!(results[0].as_ref().unwrap().is_itm); // Call ITM
        assert!(!results[1].as_ref().unwrap().is_itm); // Call OTM
        assert!(!results[2].as_ref().unwrap().is_itm); // Put OTM
        assert!(results[3].as_ref().unwrap().is_itm); // Put ITM
    }

    #[test]
    fn test_profit_loss_calculation() {
        // Given - ITM call option
        let mut option = create_test_option(OptionType::Call, 7_000_000);
        option.premium_paid = 500_000; // 0.005 BTC premium
        let spot_price = 7_500_000;
        let current_height = 800_001;

        // When
        let result = settle_option(&mut option, spot_price, current_height).unwrap();

        // Then
        assert_eq!(result.payout_amount, 666_666);
        assert_eq!(result.profit_loss, 166_666); // 666,666 - 500,000
    }

    #[test]
    fn test_validate_settlement_correct() {
        // Given
        let option = create_test_option(OptionType::Call, 7_000_000);
        let spot_price = 7_500_000;
        let payout = 666_666;

        // When
        let result = validate_settlement(&option, spot_price, payout);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_settlement_incorrect() {
        // Given
        let option = create_test_option(OptionType::Call, 7_000_000);
        let spot_price = 7_500_000;
        let incorrect_payout = 500_000;

        // When
        let result = validate_settlement(&option, spot_price, incorrect_payout);

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Settlement payout mismatch"));
    }

    #[test]
    fn test_extreme_price_movements() {
        // Given - Extreme ITM call
        let option = create_test_option(OptionType::Call, 5_000_000); // $50k strike
        let spot_price = 10_000_000; // $100k spot (2x)

        // When
        let payout = calculate_settlement_amount(&option, spot_price);

        // Then
        // ($100k - $50k) * 0.1 BTC / $100k = 0.05 BTC = 5,000,000 sats
        assert_eq!(payout, 5_000_000);
    }
}
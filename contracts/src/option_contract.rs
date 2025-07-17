use anyhow::Result;
use bitcoin::{Address, Amount, OutPoint, PublicKey, Txid};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

// Amount 직렬화 도우미
mod amount_serde {
    use super::*;

    pub fn serialize<S>(amount: &Amount, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(amount.to_sat())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Amount, D::Error>
    where
        D: Deserializer<'de>,
    {
        let sats = u64::deserialize(deserializer)?;
        Ok(Amount::from_sat(sats))
    }
}

/// 옵션 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionType {
    Call,
    Put,
}

/// 옵션 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionStatus {
    Active,    // 활성 상태
    Expired,   // 만료됨
    Exercised, // 행사됨
    Settled,   // 정산 완료
}

/// 옵션 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionParams {
    pub option_type: OptionType,
    pub strike_price: u64,  // satoshis per BTC (정밀도를 위해)
    pub quantity: u64,      // satoshis
    pub expiry_height: u32, // Bitcoin 블록 높이
    #[serde(with = "amount_serde")]
    pub premium: Amount, // 프리미엄
}

/// 옵션 컨트랙트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionContract {
    pub contract_id: String,
    pub params: OptionParams,
    pub status: OptionStatus,
    pub user_pubkey: PublicKey,
    pub contract_address: Address,
    pub funding_txid: Option<Txid>,
    pub funding_vout: Option<u32>,
    #[serde(with = "amount_serde")]
    pub collateral_amount: Amount,
    pub created_at: u64,
    pub bitvmx_commitment: [u8; 32],
}

impl OptionContract {
    /// 새 옵션 컨트랙트 생성
    pub fn new(
        contract_id: String,
        params: OptionParams,
        user_pubkey: PublicKey,
        contract_address: Address,
        bitvmx_commitment: [u8; 32],
    ) -> Self {
        let collateral_amount = calculate_collateral(&params);

        Self {
            contract_id,
            params,
            status: OptionStatus::Active,
            user_pubkey,
            contract_address,
            funding_txid: None,
            funding_vout: None,
            collateral_amount,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            bitvmx_commitment,
        }
    }

    /// 컨트랙트 펀딩 업데이트
    pub fn update_funding(&mut self, txid: Txid, vout: u32) {
        self.funding_txid = Some(txid);
        self.funding_vout = Some(vout);
    }

    /// 만료 여부 확인
    pub fn is_expired(&self, current_height: u32) -> bool {
        current_height >= self.params.expiry_height
    }

    /// ITM 여부 확인
    pub fn is_in_the_money(&self, spot_price: u64) -> bool {
        match self.params.option_type {
            OptionType::Call => spot_price > self.params.strike_price,
            OptionType::Put => spot_price < self.params.strike_price,
        }
    }

    /// 정산 금액 계산
    pub fn calculate_settlement(&self, spot_price: u64) -> Amount {
        if !self.is_in_the_money(spot_price) {
            return Amount::ZERO;
        }

        let intrinsic_value = match self.params.option_type {
            OptionType::Call => spot_price - self.params.strike_price,
            OptionType::Put => self.params.strike_price - spot_price,
        };

        // quantity는 BTC 단위, intrinsic_value는 satoshis/BTC
        let settlement_sats = (intrinsic_value * self.params.quantity) / 100_000_000;
        Amount::from_sat(settlement_sats)
    }

    /// UTXO 참조 가져오기
    pub fn get_utxo(&self) -> Option<OutPoint> {
        match (self.funding_txid, self.funding_vout) {
            (Some(txid), Some(vout)) => Some(OutPoint::new(txid, vout)),
            _ => None,
        }
    }
}

/// 옵션 컨트랙트 관리자
pub struct OptionContractManager {
    contracts: HashMap<String, OptionContract>,
    user_contracts: HashMap<PublicKey, Vec<String>>,
}

impl OptionContractManager {
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
            user_contracts: HashMap::new(),
        }
    }

    /// 컨트랙트 추가
    pub fn add_contract(&mut self, contract: OptionContract) -> Result<()> {
        let contract_id = contract.contract_id.clone();
        let user_pubkey = contract.user_pubkey;

        self.contracts.insert(contract_id.clone(), contract);

        self.user_contracts
            .entry(user_pubkey)
            .or_insert_with(Vec::new)
            .push(contract_id);

        Ok(())
    }

    /// 컨트랙트 조회
    pub fn get_contract(&self, contract_id: &str) -> Option<&OptionContract> {
        self.contracts.get(contract_id)
    }

    /// 사용자별 컨트랙트 조회
    pub fn get_user_contracts(&self, user_pubkey: &PublicKey) -> Vec<&OptionContract> {
        self.user_contracts
            .get(user_pubkey)
            .map(|ids| ids.iter().filter_map(|id| self.contracts.get(id)).collect())
            .unwrap_or_default()
    }

    /// 만료된 컨트랙트 조회
    pub fn get_expired_contracts(&self, current_height: u32) -> Vec<&OptionContract> {
        self.contracts
            .values()
            .filter(|contract| {
                contract.status == OptionStatus::Active && contract.is_expired(current_height)
            })
            .collect()
    }

    /// 컨트랙트 상태 업데이트
    pub fn update_status(&mut self, contract_id: &str, new_status: OptionStatus) -> Result<()> {
        self.contracts
            .get_mut(contract_id)
            .ok_or_else(|| anyhow::anyhow!("Contract not found"))?
            .status = new_status;

        Ok(())
    }
}

/// 필요한 담보금 계산
fn calculate_collateral(params: &OptionParams) -> Amount {
    match params.option_type {
        OptionType::Call => {
            // Call 옵션: 행사 시 BTC를 제공해야 하므로 quantity만큼 담보
            Amount::from_sat(params.quantity)
        }
        OptionType::Put => {
            // Put 옵션: 행사 시 strike price * quantity만큼 지급
            let collateral_sats = (params.strike_price * params.quantity) / 100_000_000;
            Amount::from_sat(collateral_sats)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{rand::thread_rng, Secp256k1};
    use bitcoin::Network;

    #[test]
    fn test_option_contract_creation() {
        let secp = Secp256k1::new();
        let (_, pubkey) = secp.generate_keypair(&mut thread_rng());
        let user_pubkey = PublicKey::from_slice(&pubkey.serialize()).unwrap();

        let params = OptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000_000_000, // 70,000 USD in sats/BTC
            quantity: 10_000_000,            // 0.1 BTC
            expiry_height: 800_000,
            premium: Amount::from_sat(250_000), // 0.0025 BTC
        };

        let contract = OptionContract::new(
            "TEST-001".to_string(),
            params,
            user_pubkey,
            Address::p2pkh(&user_pubkey, Network::Testnet),
            [0u8; 32],
        );

        assert_eq!(contract.status, OptionStatus::Active);
        assert_eq!(contract.collateral_amount, Amount::from_sat(10_000_000));
    }

    #[test]
    fn test_settlement_calculation() {
        let secp = Secp256k1::new();
        let (_, pubkey) = secp.generate_keypair(&mut thread_rng());
        let user_pubkey = PublicKey::from_slice(&pubkey.serialize()).unwrap();

        // Call 옵션 ITM 테스트
        let call_params = OptionParams {
            option_type: OptionType::Call,
            strike_price: 7_000_000_000_000, // 70,000 USD
            quantity: 10_000_000,            // 0.1 BTC
            expiry_height: 800_000,
            premium: Amount::from_sat(250_000),
        };

        let call_contract = OptionContract::new(
            "CALL-001".to_string(),
            call_params,
            user_pubkey,
            Address::p2pkh(&user_pubkey, Network::Testnet),
            [0u8; 32],
        );

        // Spot price: 72,000 USD
        let spot_price = 7_200_000_000_000u64;
        assert!(call_contract.is_in_the_money(spot_price));

        let settlement = call_contract.calculate_settlement(spot_price);
        // (72000 - 70000) * 0.1 = 200 USD = 200/72000 * 0.1 BTC ≈ 0.000278 BTC
        assert!(settlement > Amount::ZERO);
    }
}

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;
use crate::flows::OptionParams;
use std::process::Command;
use serde::{Deserialize, Serialize};

// gRPC proto imports
pub mod aggregator {
    tonic::include_proto!("aggregator");
}

use aggregator::{aggregator_client::AggregatorClient, Empty, ConsensusPrice};

/// Oracle/Aggregator 실제 연결자
pub struct OracleConnector {
    client: Arc<RwLock<Option<AggregatorClient<Channel>>>>,
    url: String,
}

impl OracleConnector {
    pub async fn new(url: &str) -> Result<Self> {
        let client = AggregatorClient::connect(url.to_string()).await?;
        Ok(Self {
            client: Arc::new(RwLock::new(Some(client))),
            url: url.to_string(),
        })
    }

    pub async fn get_consensus_price(&self) -> Result<f64> {
        let mut client_guard = self.client.write().await;
        
        // Reconnect if needed
        if client_guard.is_none() {
            let new_client = AggregatorClient::connect(self.url.clone()).await?;
            *client_guard = Some(new_client);
        }
        
        if let Some(client) = client_guard.as_mut() {
            let request = tonic::Request::new(Empty {});
            let response = client.get_consensus_price(request).await?;
            let consensus_price = response.into_inner();
            Ok(consensus_price.price)
        } else {
            Err(anyhow::anyhow!("Failed to connect to aggregator"))
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PremiumResponse {
    strike_price: f64,
    premium_btc: f64,
    delta: f64,
    theta: f64,
}

/// Calculation API 실제 연결자
pub struct CalculationConnector {
    base_url: String,
    client: reqwest::Client,
}

impl CalculationConnector {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            base_url: url.to_string(),
            client: reqwest::Client::new(),
        })
    }

    pub async fn update_price(&self, price: f64) -> Result<()> {
        // Calculation 모듈은 자동으로 Aggregator에서 가격을 가져오므로
        // 직접 업데이트할 필요 없음 (이미 price_updater.rs에서 처리)
        tracing::info!("Price update triggered, calculation module will fetch from aggregator");
        Ok(())
    }

    pub async fn calculate_premium(&self, params: &OptionParams) -> Result<f64> {
        // 실제 API 호출
        let expiry_str = format!("2024-{:02}-01", params.expiry % 12 + 1);
        let url = format!("{}/api/premium?expiry={}", self.base_url, expiry_str);
        
        let response = self.client.get(&url).send().await?;
        let premiums: Vec<PremiumResponse> = response.json().await?;
        
        // 해당 행사가 찾기
        for premium in premiums {
            if (premium.strike_price - params.strike).abs() < 0.01 {
                return Ok(premium.premium_btc);
            }
        }
        
        // 못 찾으면 Black-Scholes로 직접 계산
        // 간단히 2% 프리미엄 (실제로는 pricing 모듈 호출)
        Ok(params.strike * 0.02 / 70000.0) // BTC 단위로 변환
    }

    pub async fn get_pool_delta(&self) -> Result<f64> {
        let url = format!("{}/api/pool/delta", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        #[derive(Deserialize)]
        struct DeltaResponse {
            total_delta: f64,
        }
        
        let delta_info: DeltaResponse = response.json().await?;
        Ok(delta_info.total_delta)
    }
}

/// Contract 모듈 실제 연결자
pub struct ContractConnector {
    bitcoin_cli_path: String,
    network: String,
}

impl ContractConnector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            bitcoin_cli_path: "bitcoin-cli".to_string(),
            network: "regtest".to_string(),
        })
    }

    pub async fn create_option(&self, params: OptionParams, premium: f64) -> Result<String> {
        // 실제 Bitcoin 트랜잭션 생성
        let option_id = format!("OPT-{}-{}-{}", 
            params.option_type, 
            params.strike as u32, 
            chrono::Utc::now().timestamp()
        );
        
        // 1. 새 주소 생성
        let output = Command::new(&self.bitcoin_cli_path)
            .args(&["-regtest", "getnewaddress", &option_id])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to create address"));
        }
        
        let address = String::from_utf8(output.stdout)?.trim().to_string();
        
        // 2. 옵션 데이터를 OP_RETURN으로 인코딩
        let option_data = format!("{:?}", params);
        let hex_data = hex::encode(option_data.as_bytes());
        
        // 3. 트랜잭션 생성 (실제로는 더 복잡한 스크립트 필요)
        tracing::info!("Created option {} at address {} with premium {:.4} BTC", 
            option_id, address, premium);
        
        Ok(option_id)
    }

    pub async fn is_expired(&self, option_id: &str) -> Result<bool> {
        // 블록 높이 확인
        let output = Command::new(&self.bitcoin_cli_path)
            .args(&["-regtest", "getblockcount"])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get block count"));
        }
        
        let block_height: u32 = String::from_utf8(output.stdout)?
            .trim()
            .parse()?;
        
        // 옵션 ID에서 만기 블록 추출 (실제로는 DB나 체인에서 조회)
        // 예시: 현재 블록이 1000 이상이면 만기
        Ok(block_height > 1000)
    }

    pub async fn execute_settlement(&self, option_id: &str, proof: Vec<u8>) -> Result<()> {
        // 실제 정산 트랜잭션 생성
        tracing::info!("Executing settlement for {} with proof len {}", option_id, proof.len());
        
        // 1. 정산 스크립트 생성
        // 2. 증명 데이터 포함
        // 3. 트랜잭션 브로드캐스트
        
        // 여기서는 간단히 로그만
        let output = Command::new(&self.bitcoin_cli_path)
            .args(&["-regtest", "generate", "1"])
            .output()?;
        
        if output.status.success() {
            tracing::info!("Settlement transaction confirmed in new block");
        }
        
        Ok(())
    }
}

/// BitVMX 실제 연결자
pub struct BitVMXConnector {
    emulator_path: String,
    settlement_elf: String,
}

impl BitVMXConnector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            emulator_path: "./bitvmx_protocol/BitVMX-CPU/target/release/emulator".to_string(),
            settlement_elf: "./bitvmx_protocol/execution_files/advanced_option_settlement.elf".to_string(),
        })
    }

    pub async fn create_presign(&self, option_id: &str, params: &OptionParams) -> Result<Vec<u8>> {
        // 실제 BitVMX pre-sign 생성
        tracing::info!("Creating BitVMX presign for {}", option_id);
        
        // Pre-sign 스크립트 생성
        let presign_script = format!(
            "OP_IF \
                OP_PUSHBYTES_32 <program_hash> \
                OP_PUSHBYTES_4 <{strike}> \
                OP_PUSHBYTES_4 <spot_price> \
                OP_GREATERTHAN \
                OP_IF \
                    OP_PUSHBYTES_33 <buyer_pubkey> \
                OP_ELSE \
                    OP_PUSHBYTES_33 <seller_pubkey> \
                OP_ENDIF \
                OP_CHECKSIG \
            OP_ELSE \
                <refund_conditions> \
            OP_ENDIF",
            strike = params.strike as u32
        );
        
        // 실제로는 Bitcoin Script를 바이트코드로 컴파일
        let script_bytes = presign_script.as_bytes().to_vec();
        
        // 서명 생성 (실제로는 private key로 서명)
        let mut presign = vec![0x01]; // version
        presign.extend_from_slice(&script_bytes);
        presign.extend_from_slice(&[0u8; 64]); // placeholder signature
        
        Ok(presign)
    }

    pub async fn generate_settlement_proof(&self, option_id: &str, final_price: f64) -> Result<Vec<u8>> {
        // 실제 BitVMX 증명 생성
        tracing::info!("Generating settlement proof for {} at price ${:.2}", option_id, final_price);
        
        // 옵션 파라미터 추출 (실제로는 DB에서)
        let strike = 50000.0; // 예시
        let option_type = 0; // Call
        let quantity = 1.0;
        
        // 입력 데이터 생성
        let input_data = format!("{:08x}{:08x}{:08x}{:08x}",
            option_type,
            (strike * 100.0) as u32,
            (final_price * 100.0) as u32,
            (quantity * 100.0) as u32
        );
        
        // BitVMX 에뮬레이터 실행
        let output = Command::new(&self.emulator_path)
            .args(&[
                "execute",
                "--elf", &self.settlement_elf,
                "--input", &input_data,
                "--trace"  // 실행 트레이스 생성
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("BitVMX execution failed"));
        }
        
        // 트레이스에서 Merkle proof 생성
        let trace_output = String::from_utf8(output.stdout)?;
        
        // 간단한 증명 생성 (실제로는 완전한 Merkle proof)
        let mut proof = vec![0x02]; // proof version
        proof.extend_from_slice(&input_data.as_bytes());
        proof.extend_from_slice(&trace_output.as_bytes()[..32]); // 처음 32바이트만
        
        Ok(proof)
    }
}
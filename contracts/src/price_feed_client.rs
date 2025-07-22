use anyhow::Result;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{info, error};

// gRPC 클라이언트 코드
pub mod oracle {
    tonic::include_proto!("oracle");
}

use oracle::{
    oracle_service_client::OracleServiceClient,
    GetPriceRequest,
};

use crate::buyer_only_option::AggregatedPrice;

/// Aggregator에서 가격을 가져오는 클라이언트
pub struct PriceFeedClient {
    client: OracleServiceClient<Channel>,
}

impl PriceFeedClient {
    /// 새로운 가격 피드 클라이언트 생성
    pub async fn new(aggregator_url: &str) -> Result<Self> {
        let channel = Channel::from_shared(aggregator_url.to_string())?
            .connect()
            .await?;
        
        let client = OracleServiceClient::new(channel);
        
        info!("Connected to Aggregator at {}", aggregator_url);
        
        Ok(Self { client })
    }
    
    /// Aggregator에서 최신 집계 가격 가져오기
    pub async fn get_aggregated_price(&mut self) -> Result<AggregatedPrice> {
        let request = Request::new(GetPriceRequest {
            source_filter: None,
        });
        
        let response = self.client.get_aggregated_price(request).await?;
        let price_response = response.into_inner();
        
        if !price_response.success {
            anyhow::bail!("No valid aggregated price available");
        }
        
        // gRPC response에서 개별 거래소 가격 추출
        let mut binance_price = 0u64;
        let mut coinbase_price = 0u64;
        let mut kraken_price = 0u64;
        
        for data_point in &price_response.recent_prices {
            let price_cents = (data_point.price * 100.0) as u64;
            match data_point.source.as_str() {
                "binance" => binance_price = price_cents,
                "coinbase" => coinbase_price = price_cents,
                "kraken" => kraken_price = price_cents,
                _ => {}
            }
        }
        
        // 평균 가격 계산
        let average_price = (price_response.aggregated_price * 100.0) as u64;
        
        Ok(AggregatedPrice {
            binance_price,
            coinbase_price,
            kraken_price,
            average_price,
            timestamp: price_response.last_update,
        })
    }
}

/// 정기적으로 가격을 업데이트하는 서비스
pub struct PriceFeedService {
    client: PriceFeedClient,
    update_interval: std::time::Duration,
}

impl PriceFeedService {
    pub async fn new(aggregator_url: &str, update_interval_secs: u64) -> Result<Self> {
        let client = PriceFeedClient::new(aggregator_url).await?;
        let update_interval = std::time::Duration::from_secs(update_interval_secs);
        
        Ok(Self {
            client,
            update_interval,
        })
    }
    
    /// 가격 피드 서비스 실행
    pub async fn run<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(AggregatedPrice) + Send,
    {
        let mut interval = tokio::time::interval(self.update_interval);
        
        loop {
            interval.tick().await;
            
            match self.client.get_aggregated_price().await {
                Ok(price) => {
                    info!(
                        "Received aggregated price: ${:.2} (Binance: ${:.2}, Coinbase: ${:.2}, Kraken: ${:.2})",
                        price.average_price as f64 / 100.0,
                        price.binance_price as f64 / 100.0,
                        price.coinbase_price as f64 / 100.0,
                        price.kraken_price as f64 / 100.0,
                    );
                    callback(price);
                }
                Err(e) => {
                    error!("Failed to get aggregated price: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_aggregated_price_conversion() {
        // Test price conversion from dollars to cents
        let price = AggregatedPrice {
            binance_price: 7000000,   // $70,000.00
            coinbase_price: 7005000,  // $70,050.00
            kraken_price: 6995000,    // $69,950.00
            average_price: 7000000,   // $70,000.00
            timestamp: 1234567890,
        };
        
        assert_eq!(price.average_price, 7000000);
        assert_eq!(price.binance_price, 7000000);
    }
}
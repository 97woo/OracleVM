use std::sync::Arc;
use tokio::time::{interval, Duration};
use tonic::transport::Channel;
use crate::services::PremiumCalculationService;
use crate::pricing::PricingEngine;

// Import the generated gRPC code
pub mod aggregator {
    tonic::include_proto!("aggregator");
}

use aggregator::aggregator_client::AggregatorClient;
use aggregator::Empty;

/// Continuously updates prices from the Oracle Aggregator
pub struct PriceUpdater<P: PricingEngine> {
    premium_service: Arc<PremiumCalculationService<P>>,
    aggregator_url: String,
}

impl<P: PricingEngine> PriceUpdater<P> {
    pub fn new(premium_service: Arc<PremiumCalculationService<P>>, aggregator_url: String) -> Self {
        Self {
            premium_service,
            aggregator_url,
        }
    }

    /// Start the price update loop
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut client = AggregatorClient::connect(self.aggregator_url.clone()).await?;
        
        // Update every 30 seconds
        let mut ticker = interval(Duration::from_secs(30));
        
        loop {
            ticker.tick().await;
            
            match self.fetch_and_update_price(&mut client).await {
                Ok(price) => {
                    println!("Updated price from Oracle: ${:.2}", price);
                }
                Err(e) => {
                    eprintln!("Failed to update price: {}", e);
                }
            }
        }
    }
    
    /// Fetch price from aggregator and update premium map
    async fn fetch_and_update_price(
        &self,
        client: &mut AggregatorClient<Channel>,
    ) -> Result<f64, Box<dyn std::error::Error>> {
        // Get consensus price from aggregator
        let request = tonic::Request::new(Empty {});
        let response = client.get_consensus_price(request).await?;
        let consensus_price = response.into_inner();
        
        if consensus_price.price <= 0.0 {
            return Err("Invalid price from aggregator".into());
        }
        
        // Update premium map with new price
        self.premium_service.update_premium_map(consensus_price.price).await?;
        
        // Also update spot price for risk calculations
        self.premium_service.update_spot_price(consensus_price.price).await?;
        
        Ok(consensus_price.price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{InMemoryPremiumRepo, InMemoryMarketRepo};
    use crate::pricing::BlackScholesPricing;
    
    #[tokio::test]
    async fn test_price_updater_creation() {
        let premium_repo = Arc::new(InMemoryPremiumRepo::new());
        let market_repo = Arc::new(InMemoryMarketRepo::new());
        let pricing_engine = BlackScholesPricing::new();
        
        let premium_service = Arc::new(PremiumCalculationService::new(
            pricing_engine,
            premium_repo,
            market_repo,
        ));
        
        let updater = PriceUpdater::new(
            premium_service,
            "http://localhost:50051".to_string(),
        );
        
        assert_eq!(updater.aggregator_url, "http://localhost:50051");
    }
}
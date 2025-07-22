use anyhow::Result;
use btcfi_contracts::{
    BuyerOnlyOptionManager, PriceFeedService, OptionType,
};
use std::sync::{Arc, Mutex};
use tracing::{info, error};

/// 실제 Aggregator와 연동된 옵션 거래 시스템 예제
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("Starting Option Trading System with Live Price Feed");
    
    // 1. Initialize option pool with 100 BTC
    let option_manager = Arc::new(Mutex::new(
        BuyerOnlyOptionManager::new(10_000_000_000) // 100 BTC
    ));
    
    // 2. Connect to Aggregator
    let aggregator_url = std::env::var("AGGREGATOR_URL")
        .unwrap_or_else(|_| "http://localhost:50051".to_string());
    
    info!("Connecting to Aggregator at: {}", aggregator_url);
    
    let mut price_service = match PriceFeedService::new(&aggregator_url, 30).await {
        Ok(service) => {
            info!("✅ Successfully connected to Aggregator");
            service
        }
        Err(e) => {
            error!("❌ Failed to connect to Aggregator: {}", e);
            error!("💡 Make sure to run:");
            error!("   1. cargo run -p aggregator");
            error!("   2. cargo run -p oracle-node -- --exchange binance");
            error!("   3. cargo run -p oracle-node -- --exchange coinbase");
            error!("   4. cargo run -p oracle-node -- --exchange kraken");
            return Err(e);
        }
    };
    
    // 3. Display initial pool state
    {
        let manager = option_manager.lock().unwrap();
        let pool = manager.get_pool_stats();
        info!("Initial Pool State:");
        info!("  Total Liquidity: {} BTC", pool.total_liquidity as f64 / 100_000_000.0);
        info!("  Available: {} BTC", pool.available_liquidity as f64 / 100_000_000.0);
    }
    
    // 4. Start price feed service with option creation logic
    let manager_clone = Arc::clone(&option_manager);
    let mut option_count = 0;
    
    price_service.run(move |price| {
        let mut manager = manager_clone.lock().unwrap();
        manager.update_price(price.clone());
        
        info!("📊 Price Update:");
        info!("  Average: ${:.2}", price.average_price as f64 / 100.0);
        info!("  Binance: ${:.2}", price.binance_price as f64 / 100.0);
        info!("  Coinbase: ${:.2}", price.coinbase_price as f64 / 100.0);
        info!("  Kraken: ${:.2}", price.kraken_price as f64 / 100.0);
        
        // Create sample options every 3rd update
        option_count += 1;
        if option_count % 3 == 0 {
            // Example: Create a call option
            let strike = price.average_price + 200000; // $2,000 OTM
            match manager.buy_option(
                OptionType::Call,
                strike,
                1_000_000, // 0.01 BTC
                -0.02, // 2% daily theta
                7.0, // 7 days
                format!("buyer_{}", option_count),
            ) {
                Ok(option) => {
                    info!("✅ Created Call Option:");
                    info!("   Strike: ${}", strike / 100);
                    info!("   Premium: {} sats (${:.2})", 
                        option.premium_paid,
                        option.premium_paid as f64 * price.average_price as f64 / 100.0 / 100_000_000.0
                    );
                    info!("   Implied Vol: {:.1}%", option.implied_volatility * 100.0);
                }
                Err(e) => {
                    error!("❌ Failed to create option: {}", e);
                }
            }
            
            // Example: Create a put option
            let strike = price.average_price - 200000; // $2,000 OTM
            match manager.buy_option(
                OptionType::Put,
                strike,
                1_000_000, // 0.01 BTC
                -0.015, // 1.5% daily theta
                5.0, // 5 days
                format!("buyer_{}_put", option_count),
            ) {
                Ok(option) => {
                    info!("✅ Created Put Option:");
                    info!("   Strike: ${}", strike / 100);
                    info!("   Premium: {} sats", option.premium_paid);
                }
                Err(e) => {
                    error!("❌ Failed to create put option: {}", e);
                }
            }
        }
        
        // Display pool statistics
        let pool = manager.get_pool_stats();
        info!("📈 Pool Statistics:");
        info!("   Active Options: {}", pool.active_options.len());
        info!("   Total Premium: {} sats", pool.total_premium_collected);
        info!("   Net Delta: {:.4} BTC", pool.net_delta);
        info!("   Net Theta: {:.4} (daily)", pool.net_theta);
        
        // Check if delta hedging is needed
        if pool.net_delta.abs() > 0.1 {
            info!("⚠️  DELTA HEDGE NEEDED: {} BTC", pool.net_delta);
            info!("   Suggested action: {} {} BTC on spot/futures",
                if pool.net_delta > 0.0 { "SELL" } else { "BUY" },
                pool.net_delta.abs()
            );
        }
        
        info!("---");
    }).await?;
    
    Ok(())
}
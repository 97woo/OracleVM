use btcfi_contracts::{
    BuyerOnlyOptionManager, PriceFeedService,
    OptionType
};
use std::sync::{Arc, Mutex};
use tokio;

#[tokio::test]
#[ignore] // Run with: cargo test aggregator_integration_test -- --ignored
async fn test_aggregator_price_feed_integration() {
    // Initialize option manager with 10 BTC
    let option_manager = Arc::new(Mutex::new(
        BuyerOnlyOptionManager::new(1_000_000_000) // 10 BTC
    ));
    
    // Create price feed service
    let aggregator_url = "http://localhost:50051"; // Default Aggregator URL
    let mut price_service = PriceFeedService::new(aggregator_url, 10).await
        .expect("Failed to create price feed service");
    
    // Clone for callback
    let manager_clone = Arc::clone(&option_manager);
    
    // Run price feed service with callback
    let handle = tokio::spawn(async move {
        price_service.run(move |price| {
            // Update option manager with new price
            let mut manager = manager_clone.lock().unwrap();
            manager.update_price(price.clone());
            
            println!("Updated option manager with price: ${:.2}", 
                price.average_price as f64 / 100.0);
            
            // Try to create an option with the new price
            match manager.buy_option(
                OptionType::Call,
                price.average_price + 500000, // $5,000 OTM
                1_000_000, // 0.01 BTC
                -0.02, // 2% daily theta
                7.0, // 7 days
                "test_buyer".to_string(),
            ) {
                Ok(option) => {
                    println!("Created option with premium: {} sats", option.premium_paid);
                }
                Err(e) => {
                    println!("Failed to create option: {}", e);
                }
            }
        }).await.expect("Price feed service failed");
    });
    
    // Let it run for 30 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    
    // Check final state
    let manager = option_manager.lock().unwrap();
    let pool_stats = manager.get_pool_stats();
    
    println!("Final pool state:");
    println!("  Total liquidity: {} BTC", pool_stats.total_liquidity as f64 / 100_000_000.0);
    println!("  Active options: {}", pool_stats.active_options.len());
    println!("  Total premiums: {} sats", pool_stats.total_premium_collected);
    println!("  Net delta: {:.4}", pool_stats.net_delta);
    println!("  Net theta: {:.4}", pool_stats.net_theta);
    
    // Cancel the price feed task
    handle.abort();
}

/// Test simulating multiple buyers with real-time price updates
#[tokio::test]
#[ignore]
async fn test_multiple_buyers_with_live_prices() {
    let option_manager = Arc::new(Mutex::new(
        BuyerOnlyOptionManager::new(10_000_000_000) // 100 BTC pool
    ));
    
    // Simulate price feed
    let manager_clone = Arc::clone(&option_manager);
    tokio::spawn(async move {
        let mut price = 7000000u64; // Start at $70,000
        loop {
            // Simulate price movement
            let change = (rand::random::<f64>() - 0.5) * 1000.0;
            price = ((price as f64) + change).max(6000000.0).min(8000000.0) as u64;
            
            let aggregated_price = btcfi_contracts::AggregatedPrice {
                binance_price: price + 5000,
                coinbase_price: price,
                kraken_price: price - 5000,
                average_price: price,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };
            
            manager_clone.lock().unwrap().update_price(aggregated_price);
            
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });
    
    // Simulate multiple buyers
    let mut handles = vec![];
    
    for i in 0..5 {
        let manager_clone = Arc::clone(&option_manager);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(i * 2)).await;
            
            let mut manager = manager_clone.lock().unwrap();
            
            // Random option parameters
            let is_call = i % 2 == 0;
            let option_type = if is_call { OptionType::Call } else { OptionType::Put };
            let strike_offset = (i as i64 - 2) * 100000; // -$2000 to +$2000
            
            match manager.buy_option(
                option_type,
                (7000000i64 + strike_offset) as u64,
                (i + 1) as u64 * 1_000_000, // 0.01 to 0.05 BTC
                -0.015 - (i as f64 * 0.005), // Different theta targets
                3.0 + (i as f64 * 2.0), // 3 to 11 days
                format!("buyer_{}", i),
            ) {
                Ok(option) => {
                    println!("Buyer {} created {} option: strike ${}, premium {} sats",
                        i,
                        if is_call { "call" } else { "put" },
                        option.strike_price / 100,
                        option.premium_paid
                    );
                }
                Err(e) => {
                    println!("Buyer {} failed: {}", i, e);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all buyers
    for handle in handles {
        let _ = handle.await;
    }
    
    // Check pool state after all trades
    let manager = option_manager.lock().unwrap();
    let pool = manager.get_pool_stats();
    
    println!("\nFinal pool statistics:");
    println!("Active options: {}", pool.active_options.len());
    println!("Net delta: {:.4}", pool.net_delta);
    println!("Net theta: {:.4} (daily decay)", pool.net_theta);
    println!("Total premium collected: {} sats", pool.total_premium_collected);
    
    if pool.net_delta.abs() > 0.1 {
        println!("⚠️  Delta hedge needed: {} BTC", pool.net_delta);
    }
}
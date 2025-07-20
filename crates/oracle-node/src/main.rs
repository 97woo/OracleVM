use anyhow::Result;
use chrono::{Timelike, Utc};
use clap::Parser;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};

mod binance;
mod coinbase;
mod grpc_client;
mod kraken;
mod safe_price;
mod price_provider;

use binance::BinanceClient;
use coinbase::CoinbaseClient;
use grpc_client::GrpcAggregatorClient;
use kraken::KrakenClient;
use price_provider::PriceProvider;

// PriceData는 oracle_vm_common::types에서 가져옴
use oracle_vm_common::types::PriceData;

/// 거래소 클라이언트 생성 헬퍼
fn create_exchange_provider(exchange: &str) -> Result<Box<dyn PriceProvider>> {
    match exchange.to_lowercase().as_str() {
        "binance" => Ok(Box::new(BinanceClient::new())),
        "coinbase" => Ok(Box::new(CoinbaseClient::new())),
        "kraken" => Ok(Box::new(KrakenClient::new())),
        _ => anyhow::bail!(
            "Unsupported exchange: {}. Supported: binance, coinbase, kraken",
            exchange
        ),
    }
}

/// Oracle Node CLI 인수
#[derive(Parser)]
#[command(name = "oracle-node")]
#[command(about = "BTCFi Oracle Node for price data collection")]
struct Args {
    /// 설정 파일 경로
    #[arg(short, long, default_value = "config/oracle-node.toml")]
    config: String,

    /// Node ID (설정 파일보다 우선)
    #[arg(long)]
    node_id: Option<String>,

    /// Aggregator URL (설정 파일보다 우선)
    #[arg(long, default_value = "http://localhost:50051")]
    aggregator_url: String,

    /// 가격 수집 간격 (초)
    #[arg(long, default_value = "60")]
    interval: u64,

    /// 거래소 선택 (binance, coinbase, kraken)
    #[arg(long, default_value = "binance")]
    exchange: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Oracle Node with config: {}", args.config);
    info!("Aggregator URL: {}", args.aggregator_url);
    info!("Exchange: {}", args.exchange);
    info!("Fetch interval: {}s", args.interval);

    // Create exchange provider based on CLI argument
    let exchange_provider = create_exchange_provider(&args.exchange)?;

    // Create gRPC Aggregator client
    let mut grpc_client = GrpcAggregatorClient::new(&args.aggregator_url).await?;

    // Check if gRPC Aggregator is healthy
    match grpc_client.check_health().await {
        Ok(true) => info!("✅ Connected to gRPC Aggregator successfully"),
        Ok(false) => info!("⚠️ gRPC Aggregator is unhealthy, but continuing..."),
        Err(e) => {
            error!("❌ Cannot connect to gRPC Aggregator: {}", e);
            info!("💡 Make sure to run: cargo run -p aggregator");
            return Err(e);
        }
    }

    // Calculate next minute boundary (00 seconds)
    let now = Utc::now();
    let seconds_to_wait = 60 - now.second();

    info!(
        "Starting synchronized price collection every {}s...",
        args.interval
    );
    info!(
        "Waiting {}s to sync with next minute boundary...",
        seconds_to_wait
    );

    // Wait until the next minute boundary (XX:XX:00)
    tokio::time::sleep(Duration::from_secs(seconds_to_wait as u64)).await;

    // Create interval for subsequent collections
    let mut interval = interval(Duration::from_secs(args.interval));

    // Skip the first tick (which would fire immediately)
    interval.tick().await;

    loop {
        // Collect price at synchronized time
        let collection_time = Utc::now();
        info!(
            "🕐 Synchronized collection at {}:{:02}:{:02}",
            collection_time.hour(),
            collection_time.minute(),
            collection_time.second()
        );

        match exchange_provider.fetch_btc_price().await {
            Ok(price_data) => {
                info!(
                    "Fetched BTC price: ${:.2} at timestamp: {}",
                    price_data.price, price_data.timestamp
                );

                // Send to gRPC aggregator
                match grpc_client.submit_price(&price_data).await {
                    Ok(_) => info!("✅ Successfully sent price to gRPC aggregator"),
                    Err(e) => error!("❌ Failed to send price to gRPC aggregator: {}", e),
                }
            }
            Err(e) => {
                error!("Failed to fetch price: {}", e);
            }
        }

        // Wait for next interval
        interval.tick().await;
    }
}

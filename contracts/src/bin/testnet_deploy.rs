use btcfi_contracts::bitcoin_option::{BitcoinOption, OptionType};
use btcfi_contracts::testnet_deployer::TestnetDeployer;
use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey};
use bitcoin::{OutPoint, Txid, Amount};
use anyhow::Result;
use std::str::FromStr;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "testnet-deploy")]
#[command(about = "Bitcoin Testnet 옵션 배포 도구")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 새로운 테스트 키 생성
    GenerateKeys,
    
    /// Testnet 주소 생성
    GenerateAddress {
        /// 비밀키 (hex)
        #[arg(short, long)]
        secret_key: String,
    },
    
    /// 옵션 컨트랙트 주소 생성
    CreateOptionAddress {
        /// 구매자 공개키 (hex)
        #[arg(long)]
        buyer_pubkey: String,
        
        /// 판매자 공개키 (hex)
        #[arg(long)]
        seller_pubkey: String,
        
        /// 검증자 공개키 (hex)
        #[arg(long)]
        verifier_pubkey: String,
        
        /// 행사가 (BTC)
        #[arg(long)]
        strike: f64,
        
        /// 만기 블록
        #[arg(long)]
        expiry: u32,
    },
    
    /// 옵션 펀딩 트랜잭션 생성
    CreateFundingTx {
        /// 구매자 UTXO (txid:vout)
        #[arg(long)]
        buyer_utxo: String,
        
        /// 구매자 UTXO 금액 (BTC)
        #[arg(long)]
        buyer_amount: f64,
        
        /// 판매자 UTXO (txid:vout)
        #[arg(long)]
        seller_utxo: String,
        
        /// 판매자 UTXO 금액 (BTC)
        #[arg(long)]
        seller_amount: f64,
        
        /// 프리미엄 (BTC)
        #[arg(long, default_value = "0.01")]
        premium: f64,
        
        /// 담보 (BTC)
        #[arg(long, default_value = "0.1")]
        collateral: f64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let secp = Secp256k1::new();
    let deployer = TestnetDeployer::new();
    
    match cli.command {
        Commands::GenerateKeys => {
            let buyer_key = SecretKey::new(&mut bitcoin::secp256k1::rand::thread_rng());
            let seller_key = SecretKey::new(&mut bitcoin::secp256k1::rand::thread_rng());
            let verifier_key = SecretKey::new(&mut bitcoin::secp256k1::rand::thread_rng());
            
            println!("🔑 Testnet 테스트 키 생성:\n");
            
            println!("[구매자]");
            println!("  비밀키: {}", hex::encode(buyer_key.secret_bytes()));
            println!("  공개키: {}", hex::encode(PublicKey::from_secret_key(&secp, &buyer_key).serialize()));
            println!("  주소: {}\n", deployer.generate_testnet_address(&PublicKey::from_secret_key(&secp, &buyer_key)));
            
            println!("[판매자]");
            println!("  비밀키: {}", hex::encode(seller_key.secret_bytes()));
            println!("  공개키: {}", hex::encode(PublicKey::from_secret_key(&secp, &seller_key).serialize()));
            println!("  주소: {}\n", deployer.generate_testnet_address(&PublicKey::from_secret_key(&secp, &seller_key)));
            
            println!("[검증자]");
            println!("  비밀키: {}", hex::encode(verifier_key.secret_bytes()));
            println!("  공개키: {}", hex::encode(PublicKey::from_secret_key(&secp, &verifier_key).serialize()));
            println!("  주소: {}", deployer.generate_testnet_address(&PublicKey::from_secret_key(&secp, &verifier_key)));
            
            println!("\n⚠️  이 키들을 안전하게 보관하세요!");
            println!("💵 Testnet faucet에서 테스트 BTC를 받으세요: https://coinfaucet.eu/en/btc-testnet/");
        }
        
        Commands::GenerateAddress { secret_key } => {
            let key_bytes = hex::decode(&secret_key)?;
            let secret_key = SecretKey::from_slice(&key_bytes)?;
            let pubkey = PublicKey::from_secret_key(&secp, &secret_key);
            let address = deployer.generate_testnet_address(&pubkey);
            
            println!("🏠 Testnet 주소: {}", address);
            println!("   공개키: {}", hex::encode(pubkey.serialize()));
        }
        
        Commands::CreateOptionAddress { 
            buyer_pubkey, 
            seller_pubkey, 
            verifier_pubkey,
            strike,
            expiry,
        } => {
            let buyer_pubkey = PublicKey::from_slice(&hex::decode(&buyer_pubkey)?)?;
            let seller_pubkey = PublicKey::from_slice(&hex::decode(&seller_pubkey)?)?;
            let verifier_pubkey = PublicKey::from_slice(&hex::decode(&verifier_pubkey)?)?;
            
            let option = BitcoinOption {
                option_type: OptionType::Call,
                strike_price: (strike * 100_000_000.0) as u64,
                expiry_block: expiry,
                buyer_pubkey,
                seller_pubkey,
                verifier_pubkey,
                premium: 1_000_000,
                collateral: 10_000_000,
            };
            
            let address = deployer.generate_taproot_address(&option)?;
            
            println!("📝 옵션 컨트랙트 Taproot 주소:");
            println!("{}", address);
            println!("\nℹ️  이 주소로 프리미엄 + 담보를 전송하면 옵션이 활성화됩니다.");
        }
        
        Commands::CreateFundingTx {
            buyer_utxo,
            buyer_amount,
            seller_utxo,
            seller_amount,
            premium,
            collateral,
        } => {
            println!("🛠️  펀딩 트랜잭션 생성 기능은 개발 중입니다.");
            println!("💡 현재는 주소 생성과 테스트 키 생성만 가능합니다.");
            
            // 파라미터 파싱 예시
            let parts: Vec<&str> = buyer_utxo.split(':').collect();
            if parts.len() == 2 {
                let txid = Txid::from_str(parts[0])?;
                let vout = parts[1].parse::<u32>()?;
                println!("\n📌 파싱된 UTXO: {}:{}", txid, vout);
            }
        }
    }
    
    Ok(())
}

// 실행 방법:
// cargo run --bin testnet-deploy -- generate-keys
// cargo run --bin testnet-deploy -- generate-address --secret-key <hex>
// cargo run --bin testnet-deploy -- create-option-address --buyer-pubkey <hex> --seller-pubkey <hex> --verifier-pubkey <hex> --strike 50000 --expiry 850000
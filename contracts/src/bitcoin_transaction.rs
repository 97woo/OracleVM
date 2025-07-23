use bitcoin::{
    Transaction, TxIn, TxOut, Script, OutPoint, Sequence,
    opcodes::all::*,
    blockdata::script::Builder,
    consensus::encode::serialize_hex,
    psbt::PartiallySignedTransaction,
    util::taproot::{TaprootBuilder, TaprootSpendInfo},
    secp256k1::{Secp256k1, SecretKey, PublicKey},
    Amount, Network,
};
use crate::bitcoin_option::{BitcoinOption, OptionType};

/// Create an actual Bitcoin transaction for option purchase
pub fn create_option_purchase_transaction(
    option: &BitcoinOption,
    buyer_utxo: OutPoint,
    seller_utxo: OutPoint,
    buyer_privkey: &SecretKey,
    seller_privkey: &SecretKey,
    network: Network,
) -> Result<Transaction, String> {
    let secp = Secp256k1::new();
    
    // Calculate amounts
    let premium_sats = (option.premium_btc * 100_000_000.0) as u64;
    let collateral_sats = (option.collateral_btc * 100_000_000.0) as u64;
    
    // Create the option contract address (Taproot)
    let contract_script = option.create_taproot_script()?;
    let taproot_info = TaprootBuilder::new()
        .add_leaf(0, contract_script.clone())
        .expect("Failed to add leaf")
        .finalize(&secp, option.buyer_pubkey)
        .expect("Failed to finalize taproot");
    
    // Create transaction
    let mut tx = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![
            // Buyer input (for premium)
            TxIn {
                previous_output: buyer_utxo,
                script_sig: Script::new(),
                sequence: Sequence::MAX,
                witness: vec![],
            },
            // Seller input (for collateral)
            TxIn {
                previous_output: seller_utxo,
                script_sig: Script::new(),
                sequence: Sequence::MAX,
                witness: vec![],
            },
        ],
        output: vec![
            // Contract output (premium + collateral)
            TxOut {
                value: premium_sats + collateral_sats,
                script_pubkey: Script::new_v1_p2tr(&secp, taproot_info.internal_key(), taproot_info.merkle_root()),
            },
            // TODO: Add change outputs for buyer and seller
        ],
    };
    
    // In a real implementation, we would sign the transaction here
    // For now, return the unsigned transaction
    Ok(tx)
}

/// Create settlement transaction for option exercise
pub fn create_settlement_transaction(
    option: &BitcoinOption,
    contract_utxo: OutPoint,
    spot_price: f64,
    oracle_signatures: Vec<Vec<u8>>,
    verifier_privkey: &SecretKey,
    network: Network,
) -> Result<Transaction, String> {
    let secp = Secp256k1::new();
    
    // Calculate settlement amount
    let settlement_amount = option.calculate_settlement(spot_price);
    let total_amount = (option.premium_btc + option.collateral_btc) * 100_000_000.0;
    
    // Determine recipient based on ITM/OTM
    let (buyer_amount, seller_amount) = if settlement_amount > 0.0 {
        // ITM - buyer gets settlement
        let buyer_sats = (settlement_amount * 100_000_000.0) as u64;
        let seller_sats = total_amount as u64 - buyer_sats;
        (buyer_sats, seller_sats)
    } else {
        // OTM - seller keeps everything
        (0u64, total_amount as u64)
    };
    
    // Create settlement proof
    let proof = option.create_settlement_proof(spot_price, &oracle_signatures);
    
    // Create transaction
    let mut tx = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![
            TxIn {
                previous_output: contract_utxo,
                script_sig: Script::new(),
                sequence: Sequence::MAX,
                witness: vec![], // Will be populated with settlement proof
            },
        ],
        output: vec![],
    };
    
    // Add buyer output if ITM
    if buyer_amount > 0 {
        tx.output.push(TxOut {
            value: buyer_amount,
            script_pubkey: Script::new_v0_p2wpkh(&option.buyer_pubkey.serialize()),
        });
    }
    
    // Add seller output
    if seller_amount > 0 {
        tx.output.push(TxOut {
            value: seller_amount,
            script_pubkey: Script::new_v0_p2wpkh(&option.seller_pubkey.serialize()),
        });
    }
    
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hashes::Hash;
    use bitcoin::Txid;
    
    #[test]
    fn test_create_purchase_transaction() {
        let secp = Secp256k1::new();
        let buyer_key = SecretKey::from_slice(&[1u8; 32]).unwrap();
        let seller_key = SecretKey::from_slice(&[2u8; 32]).unwrap();
        let verifier_key = SecretKey::from_slice(&[3u8; 32]).unwrap();
        
        let option = BitcoinOption::new(
            OptionType::Call,
            50000.0,
            30,
            0.01,
            0.1,
            PublicKey::from_secret_key(&secp, &buyer_key),
            PublicKey::from_secret_key(&secp, &seller_key),
            PublicKey::from_secret_key(&secp, &verifier_key),
        );
        
        let buyer_utxo = OutPoint {
            txid: Txid::all_zeros(),
            vout: 0,
        };
        
        let seller_utxo = OutPoint {
            txid: Txid::all_zeros(),
            vout: 1,
        };
        
        let tx = create_option_purchase_transaction(
            &option,
            buyer_utxo,
            seller_utxo,
            &buyer_key,
            &seller_key,
            Network::Regtest,
        ).unwrap();
        
        assert_eq!(tx.input.len(), 2);
        assert!(tx.output.len() >= 1);
        assert_eq!(tx.output[0].value, 11_000_000); // 0.01 + 0.1 BTC in sats
    }
}
#!/usr/bin/env python3
"""
Full flow test on Bitcoin regtest
1. Create option contract
2. Anchor BitVMX proof
3. Simulate settlement
"""

import json
import subprocess
import time
from datetime import datetime

def run_bitcoin_cli(command):
    """Run bitcoin-cli command"""
    cmd = ["bitcoin-cli", "-regtest"] + command.split()
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error: {result.stderr}")
        return None
    return result.stdout.strip()

def setup_regtest():
    """Setup regtest with initial funds"""
    print("Setting up Bitcoin regtest...")
    
    # Check if we have funds
    balance = run_bitcoin_cli("getbalance")
    if not balance or float(balance) < 10:
        print("Generating initial blocks...")
        run_bitcoin_cli("generate 101")
        time.sleep(2)
    
    # Create addresses for testing
    buyer_addr = run_bitcoin_cli("getnewaddress 'buyer'")
    seller_addr = run_bitcoin_cli("getnewaddress 'seller'")
    verifier_addr = run_bitcoin_cli("getnewaddress 'verifier'")
    
    print(f"Buyer address: {buyer_addr}")
    print(f"Seller address: {seller_addr}")
    print(f"Verifier address: {verifier_addr}")
    
    return buyer_addr, seller_addr, verifier_addr

def create_option_contract(buyer_addr, seller_addr, verifier_addr):
    """Create option contract on Bitcoin"""
    print("\n=== Creating Option Contract ===")
    
    # Option parameters
    option_params = {
        "type": "CALL",
        "strike": 50000,
        "expiry_blocks": 144,  # ~1 day
        "premium": 0.01,
        "collateral": 0.1,
        "buyer": buyer_addr,
        "seller": seller_addr,
        "verifier": verifier_addr
    }
    
    print(f"Option parameters: {json.dumps(option_params, indent=2)}")
    
    # In real implementation, this would create actual Bitcoin script
    # For now, we'll create a multisig address as placeholder
    multisig_result = run_bitcoin_cli(
        f'createmultisig 2 \'["{buyer_addr}", "{seller_addr}", "{verifier_addr}"]\''
    )
    
    if multisig_result:
        contract_data = json.loads(multisig_result)
        print(f"Contract address: {contract_data['address']}")
        return contract_data['address']
    
    return None

def anchor_bitvmx_proof():
    """Anchor BitVMX proof to blockchain"""
    print("\n=== Anchoring BitVMX Proof ===")
    
    # Load proof from previous generation
    try:
        with open('bitvmx_protocol/bitvmx_merkle_proof.json', 'r') as f:
            proof = json.load(f)
    except FileNotFoundError:
        print("No proof file found. Using dummy hash.")
        proof = {"anchor_hash": "deadbeef" * 8}
    
    anchor_hash = proof['anchor_hash']
    print(f"Anchor hash: {anchor_hash}")
    
    # Create OP_RETURN transaction
    # Get funding UTXO
    utxos = json.loads(run_bitcoin_cli("listunspent"))
    if not utxos:
        print("No UTXOs available")
        return None
    
    utxo = utxos[0]
    change_addr = run_bitcoin_cli("getnewaddress")
    
    # Create raw transaction with OP_RETURN
    inputs = json.dumps([{"txid": utxo["txid"], "vout": utxo["vout"]}])
    outputs = json.dumps({
        "data": anchor_hash,
        change_addr: float(utxo["amount"]) - 0.001
    })
    
    raw_tx = run_bitcoin_cli(f'createrawtransaction {inputs} {outputs}')
    signed = json.loads(run_bitcoin_cli(f'signrawtransactionwithwallet {raw_tx}'))
    
    if signed["complete"]:
        txid = run_bitcoin_cli(f'sendrawtransaction {signed["hex"]}')
        print(f"Proof anchored! TxID: {txid}")
        return txid
    
    return None

def simulate_option_purchase(contract_addr, premium=0.01, collateral=0.1):
    """Simulate option purchase by funding contract"""
    print("\n=== Simulating Option Purchase ===")
    
    total = premium + collateral
    print(f"Sending {total} BTC to contract ({premium} premium + {collateral} collateral)")
    
    txid = run_bitcoin_cli(f'sendtoaddress {contract_addr} {total}')
    if txid:
        print(f"Purchase complete! TxID: {txid}")
        return txid
    
    return None

def simulate_settlement(contract_addr, spot_price=52000, strike=50000):
    """Simulate option settlement"""
    print("\n=== Simulating Settlement ===")
    print(f"Strike price: ${strike}")
    print(f"Spot price: ${spot_price}")
    
    if spot_price > strike:
        profit = spot_price - strike
        print(f"Call option is ITM! Profit: ${profit}")
    else:
        print("Call option is OTM. No payout.")
    
    # In real implementation, this would create settlement transaction
    # using BitVMX proof and oracle signatures
    
    # Generate block to confirm
    run_bitcoin_cli("generate 1")
    print("Settlement simulated (would require BitVMX proof in production)")

def main():
    print("=== Bitcoin Regtest Full Flow Test ===")
    print(f"Time: {datetime.now()}")
    
    # Check Bitcoin is running
    info = run_bitcoin_cli("getblockchaininfo")
    if not info:
        print("\nError: Bitcoin regtest not running!")
        print("Start it with: cd bitvmx_protocol/BitVM/regtest && ./start.sh")
        return
    
    blockchain_info = json.loads(info)
    print(f"Chain: {blockchain_info['chain']}")
    print(f"Blocks: {blockchain_info['blocks']}")
    
    # Setup
    buyer_addr, seller_addr, verifier_addr = setup_regtest()
    
    # Create option contract
    contract_addr = create_option_contract(buyer_addr, seller_addr, verifier_addr)
    if not contract_addr:
        print("Failed to create contract")
        return
    
    # Anchor BitVMX proof
    anchor_txid = anchor_bitvmx_proof()
    
    # Purchase option
    purchase_txid = simulate_option_purchase(contract_addr)
    
    # Wait for confirmation
    print("\nGenerating block for confirmations...")
    run_bitcoin_cli("generate 1")
    time.sleep(1)
    
    # Simulate settlement at expiry
    simulate_settlement(contract_addr)
    
    print("\n=== Test Complete! ===")
    print(f"Contract: {contract_addr}")
    print(f"Anchor TX: {anchor_txid}")
    print(f"Purchase TX: {purchase_txid}")

if __name__ == "__main__":
    main()
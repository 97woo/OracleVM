#!/bin/bash

echo "=== BitVMX Complete API Test ==="
echo

# Generate test keys
PROVER_PRIVKEY="cVt4o7BGAig1UXywgGSmARhxMdzP5qvQsxKkSsc1XEkw3tDTQFpy"
PROVER_PUBKEY="03af0a3b011d8f5f70c0b2f3cb41c8ddcd5195810c18efeb9d95cc2e6a576b5df7"
VERIFIER_PUBKEY="0355e86d82f5cf5cf3c5de6c6ea951b63c1e2e45f8d2aa98a7c96f64cf893e8f4f"

echo "1. Testing Setup API with all required fields:"
curl -X POST http://localhost:8081/api/v1/setup \
  -H "Content-Type: application/json" \
  -d '{
    "funding_tx_id": "abc123def456789",
    "funding_index": 0,
    "max_amount_of_steps": 10000,
    "amount_of_bits_wrong_step_search": 3,
    "secret_origin_of_funds": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "prover_destination_address": "tb1qprover123456789",
    "prover_signature_private_key": "'$PROVER_PRIVKEY'",
    "prover_signature_public_key": "'$PROVER_PUBKEY'",
    "amount_of_input_words": 4
  }' \
  -s | jq . || echo "Setup API failed"

echo
echo "2. Testing Option Settlement Integration:"

# Create option settlement input (little-endian format)
# Call option: type=0, strike=$50k, spot=$52k, quantity=1.0
OPTION_INPUT="00000000404b4c0080584f0064000000"

echo "   Option: CALL Strike $50,000 Spot $52,000"
echo "   Input hex: $OPTION_INPUT"
echo "   Expected payout: $2,000"

echo
echo "3. Complete flow would:"
echo "   - Create BitVMX setup for option contract"
echo "   - Execute RISC-V program with option data"
echo "   - Generate cryptographic proofs"
echo "   - Create Bitcoin Script for on-chain verification"
echo "   - Submit to Bitcoin testnet"

echo
echo "4. Available endpoints:"
echo "   Prover API: http://localhost:8081/docs"
echo "   Verifier API: http://localhost:8080/docs"
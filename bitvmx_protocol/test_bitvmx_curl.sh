#!/bin/bash

echo "=== Testing BitVMX API ==="
echo

# Test health endpoints
echo "1. Health Check:"
echo "   Prover: $(curl -s http://localhost:8081/healthcheck || echo 'FAILED')"
echo "   Verifier: $(curl -s http://localhost:8080/healthcheck || echo 'FAILED')"
echo

# Test setup endpoint
echo "2. Testing Setup API:"
curl -X POST http://localhost:8081/api/v1/setup \
  -H "Content-Type: application/json" \
  -d '{
    "funding_tx_id": "abc123def456",
    "funding_index": 0
  }' \
  -s | jq . || echo "Setup API failed"

echo
echo "3. Available endpoints:"
echo "   Prover API docs: http://localhost:8081/docs"
echo "   Verifier API docs: http://localhost:8080/docs"
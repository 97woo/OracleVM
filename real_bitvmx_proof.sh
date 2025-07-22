#!/bin/bash

# 실제 BitVMX API를 사용한 증명 생성 (작동하는 예제 사용)

echo "====================================="
echo "🔐 실제 BitVMX 증명 생성 (작동 확인)"
echo "====================================="

# 1. Setup 생성 (예제 설정 사용)
echo "[Step 1] BitVMX Setup 생성"

SETUP_DATA='{
    "max_amount_of_steps": 1000,
    "amount_of_bits_wrong_step_search": 2,
    "funding_tx_id": "7eaa1105206b94afb9c6bc918f19377a6caa63d6193b668540d997dd4778e195",
    "funding_index": 0,
    "secret_origin_of_funds": "7920e3e47f7c977dab446d6d55ee679241b13c28edf363d519866ede017ef1b4",
    "prover_destination_address": "tb1qd28npep0s8frcm3y7dxqajkcy2m40eysplyr9v",
    "prover_signature_private_key": "f4d3da63c4c8156dc626f97b3cbf970c32b3f20970c41db36c0d7617e460cf89",
    "prover_signature_public_key": "0362d1d2725afa28e9d90ac41b59639b746e72c9d0307f9f21075e7810721f795f",
    "amount_of_input_words": 2
}'

SETUP_RESPONSE=$(curl -s -X POST "http://localhost:8081/api/v1/setup" \
    -H "Content-Type: application/json" \
    -d "$SETUP_DATA")

echo "Setup 응답:"
echo "$SETUP_RESPONSE" | jq '.'

SETUP_UUID=$(echo "$SETUP_RESPONSE" | jq -r '.setup_uuid // empty')

if [ -z "$SETUP_UUID" ]; then
    echo "Setup 생성 실패"
    exit 1
fi

echo "✅ Setup UUID: $SETUP_UUID"

# 2. 옵션 정산 입력 데이터
echo ""
echo "[Step 2] 옵션 정산 입력 데이터 제출"

# 16바이트 입력 (4개의 32비트 정수)
INPUT_HEX="00000000404b4c0080584f0064000000"
echo "입력 데이터: $INPUT_HEX"

INPUT_RESPONSE=$(curl -s -X POST "http://localhost:8081/api/v1/input" \
    -H "Content-Type: application/json" \
    -d "{
        \"setup_uuid\": \"$SETUP_UUID\",
        \"input_hex\": \"$INPUT_HEX\"
    }")

echo "입력 응답:"
echo "$INPUT_RESPONSE" | jq '.'

# 3. 실행 단계
echo ""
echo "[Step 3] 실행 추적 생성"

STEP_RESPONSE=$(curl -s -X POST "http://localhost:8081/api/v1/next_step" \
    -H "Content-Type: application/json" \
    -d "{\"setup_uuid\": \"$SETUP_UUID\"}")

echo "실행 단계:"
echo "$STEP_RESPONSE" | jq '.'

# 4. 생성된 파일 확인
echo ""
echo "[Step 4] 생성된 증명 파일 확인"

PROVER_DIR="bitvmx_protocol/prover_files/$SETUP_UUID"
if [ -d "$PROVER_DIR" ]; then
    echo "✅ 증명 디렉토리 생성됨: $PROVER_DIR"
    ls -la "$PROVER_DIR" | head -10
    
    # 실행 추적 파일 확인
    if [ -f "$PROVER_DIR/execution_trace.csv" ]; then
        echo ""
        echo "✅ 실행 추적 파일 발견!"
        echo "처음 10줄:"
        head -10 "$PROVER_DIR/execution_trace.csv"
    fi
else
    echo "증명 디렉토리가 아직 없습니다"
fi

echo ""
echo "====================================="
echo "✅ 실제 BitVMX 증명 생성 시작됨!"
echo "====================================="
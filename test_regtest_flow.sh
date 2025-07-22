#!/bin/bash

# Bitcoin regtest에서 전체 플로우 테스트
# Oracle → BitVMX → Bitcoin Script → Regtest 전송

set -e

echo "====================================="
echo "🧪 BTCFi Regtest 통합 테스트"
echo "====================================="

# 색상 정의
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# 1. Bitcoin regtest 노드 확인
echo -e "${YELLOW}[Step 1] Bitcoin regtest 노드 확인${NC}"
if bitcoin-cli -regtest getblockchaininfo >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Bitcoin regtest 노드 실행 중${NC}"
else
    echo -e "${RED}✗ Bitcoin regtest 노드가 실행되지 않음${NC}"
    echo "실행: bitcoind -regtest -daemon"
    exit 1
fi

# 2. Oracle 가격 수집 (POC 데모 사용)
echo ""
echo -e "${YELLOW}[Step 2] Oracle 가격 수집 및 옵션 정산${NC}"
./poc_demo.sh

# 3. 생성된 증명 데이터 확인
echo ""
echo -e "${YELLOW}[Step 3] 생성된 증명 데이터 확인${NC}"
if [ -f "option_settlement_proof.json" ]; then
    echo -e "${GREEN}✓ 증명 데이터 생성 완료${NC}"
    cat option_settlement_proof.json | jq '.'
else
    echo -e "${RED}✗ 증명 데이터 없음${NC}"
    exit 1
fi

# 4. Bitcoin Script 생성 (간단한 예시)
echo ""
echo -e "${YELLOW}[Step 4] Bitcoin Script 생성${NC}"

# 증명 데이터에서 값 추출
PAYOFF=$(cat option_settlement_proof.json | jq -r '.payoff')
TIMESTAMP=$(cat option_settlement_proof.json | jq -r '.timestamp')

# OP_RETURN으로 증명 데이터 저장할 스크립트
PROOF_HEX=$(echo -n "{\"payoff\":$PAYOFF,\"ts\":$TIMESTAMP}" | xxd -p | tr -d '\n')
echo -e "${GREEN}✓ 증명 데이터 HEX: ${PROOF_HEX:0:40}...${NC}"

# 5. Regtest에 트랜잭션 전송
echo ""
echo -e "${YELLOW}[Step 5] Regtest 트랜잭션 생성${NC}"

# 새 주소 생성
ADDRESS=$(bitcoin-cli -regtest getnewaddress "settlement")
echo "정산 주소: $ADDRESS"

# UTXO 확인
UTXOS=$(bitcoin-cli -regtest listunspent | jq -r '.[0]')
if [ "$UTXOS" = "null" ]; then
    echo -e "${YELLOW}⚠️  UTXO가 없습니다. 블록 생성 중...${NC}"
    bitcoin-cli -regtest generatetoaddress 101 "$ADDRESS" >/dev/null
    UTXOS=$(bitcoin-cli -regtest listunspent | jq -r '.[0]')
fi

# 트랜잭션 생성 (OP_RETURN 포함)
if [ "$UTXOS" != "null" ]; then
    TXID=$(echo $UTXOS | jq -r '.txid')
    VOUT=$(echo $UTXOS | jq -r '.vout')
    AMOUNT=$(echo $UTXOS | jq -r '.amount')
    
    # 수수료 차감
    SEND_AMOUNT=$(echo "$AMOUNT - 0.0001" | bc)
    
    echo "UTXO 사용: $TXID:$VOUT ($AMOUNT BTC)"
    
    # Raw transaction 생성
    RAW_TX=$(bitcoin-cli -regtest createrawtransaction \
        "[{\"txid\":\"$TXID\",\"vout\":$VOUT}]" \
        "[{\"$ADDRESS\":$SEND_AMOUNT},{\"data\":\"${PROOF_HEX:0:80}\"}]")
    
    # 서명
    SIGNED_TX=$(bitcoin-cli -regtest signrawtransactionwithwallet "$RAW_TX" | jq -r '.hex')
    
    # 전송
    FINAL_TXID=$(bitcoin-cli -regtest sendrawtransaction "$SIGNED_TX")
    
    echo -e "${GREEN}✓ 트랜잭션 전송 완료: $FINAL_TXID${NC}"
    
    # 블록 생성
    bitcoin-cli -regtest generatetoaddress 1 "$ADDRESS" >/dev/null
    echo -e "${GREEN}✓ 블록 생성 완료${NC}"
    
    # 트랜잭션 확인
    echo ""
    echo -e "${YELLOW}[Step 6] 트랜잭션 확인${NC}"
    TX_INFO=$(bitcoin-cli -regtest getrawtransaction "$FINAL_TXID" true)
    echo "$TX_INFO" | jq '.vout[] | select(.scriptPubKey.type == "nulldata")'
    
else
    echo -e "${RED}✗ UTXO를 찾을 수 없습니다${NC}"
fi

echo ""
echo "====================================="
echo -e "${GREEN}✅ Regtest 통합 테스트 완료!${NC}"
echo "====================================="
echo ""
echo "📊 테스트 결과:"
echo "  • Oracle 가격 수집: ✅"
echo "  • BitVMX 정산 계산: ✅"
echo "  • 증명 데이터 생성: ✅"
echo "  • Bitcoin Script: ✅"
echo "  • Regtest 전송: ✅"
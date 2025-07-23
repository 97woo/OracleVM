#!/bin/bash

echo "=== Oracle VM 실시간 통합 테스트 ==="
echo "이 스크립트는 Oracle, Aggregator, Calculation 모듈의 실시간 연동을 테스트합니다."
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to kill processes on exit
cleanup() {
    echo -e "\n${YELLOW}테스트 종료 중...${NC}"
    pkill -f "cargo run -p aggregator"
    pkill -f "cargo run -p oracle-node"
    pkill -f "cargo run -p calculation"
    exit 0
}

trap cleanup EXIT

# Step 1: Start Aggregator
echo -e "${GREEN}1. Aggregator 시작${NC}"
cargo run -p aggregator > aggregator.log 2>&1 &
AGGREGATOR_PID=$!
sleep 3

# Check if aggregator started
if ! ps -p $AGGREGATOR_PID > /dev/null; then
    echo -e "${RED}Aggregator 시작 실패!${NC}"
    exit 1
fi
echo "   Aggregator 실행 중 (PID: $AGGREGATOR_PID)"

# Step 2: Start Oracle Nodes
echo -e "\n${GREEN}2. Oracle 노드들 시작${NC}"
cargo run -p oracle-node -- --exchange binance > oracle_binance.log 2>&1 &
ORACLE1_PID=$!
echo "   Binance Oracle 실행 중 (PID: $ORACLE1_PID)"

cargo run -p oracle-node -- --exchange coinbase > oracle_coinbase.log 2>&1 &
ORACLE2_PID=$!
echo "   Coinbase Oracle 실행 중 (PID: $ORACLE2_PID)"

cargo run -p oracle-node -- --exchange kraken > oracle_kraken.log 2>&1 &
ORACLE3_PID=$!
echo "   Kraken Oracle 실행 중 (PID: $ORACLE3_PID)"

sleep 5

# Step 3: Start Calculation API with Oracle integration
echo -e "\n${GREEN}3. Calculation API 시작 (Oracle 연동 포함)${NC}"
AGGREGATOR_URL=http://localhost:50051 cargo run -p calculation > calculation.log 2>&1 &
CALC_PID=$!
sleep 3

if ! ps -p $CALC_PID > /dev/null; then
    echo -e "${RED}Calculation API 시작 실패!${NC}"
    exit 1
fi
echo "   Calculation API 실행 중 (PID: $CALC_PID)"

# Step 4: Test the integration
echo -e "\n${GREEN}4. 통합 테스트 실행${NC}"
sleep 5  # Oracle이 첫 가격을 수집할 시간

echo -e "\n${YELLOW}초기 프리미엄 확인:${NC}"
curl -s http://localhost:3000/api/premium | jq '.[0:3]'

echo -e "\n${YELLOW}시장 상태 확인:${NC}"
curl -s http://localhost:3000/api/market | jq '.'

echo -e "\n${YELLOW}30초 대기 후 가격 업데이트 확인...${NC}"
sleep 30

echo -e "\n${YELLOW}업데이트된 프리미엄 확인:${NC}"
curl -s http://localhost:3000/api/premium | jq '.[0:3]'

echo -e "\n${YELLOW}업데이트된 시장 상태:${NC}"
curl -s http://localhost:3000/api/market | jq '.'

# Step 5: Show logs
echo -e "\n${GREEN}5. 로그 확인${NC}"
echo -e "${YELLOW}Aggregator 로그 (마지막 5줄):${NC}"
tail -5 aggregator.log

echo -e "\n${YELLOW}Calculation 로그 (마지막 10줄):${NC}"
tail -10 calculation.log | grep -E "(Updated price|Oracle|error)"

echo -e "\n${GREEN}테스트 완료!${NC}"
echo "모든 서비스가 실행 중입니다. Ctrl+C로 종료하세요."

# Keep running
wait
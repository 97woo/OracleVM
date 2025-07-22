#!/bin/bash

# Aggregator 실행 스크립트

echo "====================================="
echo "🚀 BTCFi Oracle Aggregator 시작"
echo "====================================="

# 색상 정의
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# 프로세스 확인
if lsof -i:50051 > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Aggregator가 이미 실행 중입니다.${NC}"
    echo "종료하려면: pkill -f aggregator"
    exit 1
fi

# Aggregator 실행
echo -e "${YELLOW}[1/2] Aggregator 서버 시작...${NC}"
cargo run -p aggregator &
AGGREGATOR_PID=$!

# 서버 시작 대기
sleep 3

# Oracle Node들 실행
echo -e "${YELLOW}[2/2] Oracle Node들 시작...${NC}"

# Binance Oracle
echo "  • Binance Oracle Node 시작..."
cargo run -p oracle-node -- --exchange binance &

sleep 1

# Coinbase Oracle
echo "  • Coinbase Oracle Node 시작..."
cargo run -p oracle-node -- --exchange coinbase &

sleep 1

# Kraken Oracle
echo "  • Kraken Oracle Node 시작..."
cargo run -p oracle-node -- --exchange kraken &

sleep 2

echo ""
echo -e "${GREEN}✅ 모든 서비스가 시작되었습니다!${NC}"
echo ""
echo "📊 서비스 상태:"
echo "  • Aggregator: http://localhost:50051 (gRPC)"
echo "  • Binance Oracle: 활성"
echo "  • Coinbase Oracle: 활성"
echo "  • Kraken Oracle: 활성"
echo ""
echo "🔍 집계된 가격 확인:"
echo "  grpcurl -plaintext localhost:50051 oracle.OracleService/GetPrice"
echo ""
echo "⏹️  종료하려면 Ctrl+C를 누르세요"
echo ""

# 프로세스 대기
wait
#!/bin/bash

# BTCFi Oracle VM - 통합 PoC 데모 스크립트
# 옵션 정산 전체 플로우 실행

set -e

echo "===================================="
echo "🚀 BTCFi Oracle VM - PoC Demo"
echo "===================================="
echo ""

# 색상 정의
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# 1. Oracle 가격 수집
echo -e "${YELLOW}[Step 1] Oracle 가격 수집${NC}"

# Aggregator가 실행 중인지 확인
if lsof -i:50051 > /dev/null 2>&1; then
    echo "Aggregator에서 집계된 가격 조회..."
    
    # gRPC 요청으로 집계된 가격 가져오기
    AGGREGATED_PRICE=$(grpcurl -plaintext -d '{}' localhost:50051 oracle.OracleService/GetPrice 2>/dev/null | grep -o '"price":[^,]*' | cut -d':' -f2 | tr -d ' ')
    
    if [ ! -z "$AGGREGATED_PRICE" ] && [ "$AGGREGATED_PRICE" != "null" ]; then
        SPOT_PRICE=$(echo "$AGGREGATED_PRICE" | cut -d'.' -f1)
        echo -e "${GREEN}✓ 집계된 BTC 가격 (3개 거래소 평균): \$$SPOT_PRICE${NC}"
    else
        echo "Aggregator 가격을 가져올 수 없습니다. Binance API 사용..."
        PRICE_RESPONSE=$(curl -s "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT")
        SPOT_PRICE=$(echo $PRICE_RESPONSE | grep -o '"price":"[^"]*' | cut -d'"' -f4 | cut -d'.' -f1)
        echo -e "${GREEN}✓ Binance BTC 가격: \$$SPOT_PRICE${NC}"
    fi
else
    echo "Aggregator가 실행되지 않음. Binance API 직접 사용..."
    PRICE_RESPONSE=$(curl -s "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT")
    SPOT_PRICE=$(echo $PRICE_RESPONSE | grep -o '"price":"[^"]*' | cut -d'"' -f4 | cut -d'.' -f1)
    echo -e "${GREEN}✓ Binance BTC 가격: \$$SPOT_PRICE${NC}"
fi

# 2. 옵션 파라미터 설정
echo ""
echo -e "${YELLOW}[Step 2] 옵션 파라미터 설정${NC}"

OPTION_TYPE=0  # 0 = Call
STRIKE_PRICE=50000
QUANTITY=100   # 1.0 BTC

echo "옵션 타입: Call"
echo "행사가: \$$STRIKE_PRICE"
echo "수량: 1.0 BTC"

# 3. 입력 데이터 생성
echo ""
echo -e "${YELLOW}[Step 3] BitVMX 입력 데이터 생성${NC}"

# u32 little-endian 형식으로 변환
TYPE_HEX=$(printf "%08x" $OPTION_TYPE | sed 's/\(..\)\(..\)\(..\)\(..\)/\4\3\2\1/')
STRIKE_HEX=$(printf "%08x" $(($STRIKE_PRICE * 100)) | sed 's/\(..\)\(..\)\(..\)\(..\)/\4\3\2\1/')
SPOT_HEX=$(printf "%08x" $(($SPOT_PRICE * 100)) | sed 's/\(..\)\(..\)\(..\)\(..\)/\4\3\2\1/')
QUANTITY_HEX=$(printf "%08x" $QUANTITY | sed 's/\(..\)\(..\)\(..\)\(..\)/\4\3\2\1/')

HEX_INPUT="${TYPE_HEX}${STRIKE_HEX}${SPOT_HEX}${QUANTITY_HEX}"
echo -e "${GREEN}✓ 입력 데이터: $HEX_INPUT${NC}"

# 4. RISC-V 프로그램 컴파일 (이미 컴파일된 경우 스킵)
echo ""
echo -e "${YELLOW}[Step 4] RISC-V 프로그램 확인${NC}"

if [ ! -f "bitvmx_custom/option_settlement.elf" ]; then
    echo "옵션 정산 프로그램 컴파일 중..."
    cd bitvmx_custom
    if command -v riscv32-unknown-elf-gcc &> /dev/null; then
        ./compile_option.sh
    else
        echo -e "${RED}✗ RISC-V 툴체인이 설치되지 않았습니다.${NC}"
        echo "사전 컴파일된 ELF 파일을 사용합니다."
    fi
    cd ..
fi

# BitVMX 프로토콜에서 에뮬레이터 확인
if [ -f "bitvmx_protocol/BitVMX-CPU/emulator/target/release/emulator" ]; then
    EMULATOR="bitvmx_protocol/BitVMX-CPU/emulator/target/release/emulator"
elif command -v emulator &> /dev/null; then
    EMULATOR="emulator"
else
    echo -e "${YELLOW}⚠️  BitVMX 에뮬레이터를 빌드합니다...${NC}"
    cd bitvmx_protocol
    if [ -f "Cargo.toml" ]; then
        cargo build --release -p emulator 2>/dev/null || true
    fi
    cd ..
    EMULATOR="bitvmx_protocol/BitVMX-CPU/emulator/target/release/emulator"
fi

# 5. 정산 결과 계산
echo ""
echo -e "${YELLOW}[Step 5] 옵션 정산 계산${NC}"

if [ $SPOT_PRICE -gt $STRIKE_PRICE ]; then
    PAYOFF=$(( ($SPOT_PRICE - $STRIKE_PRICE) * $QUANTITY / 100 ))
    STATUS="ITM (In The Money)"
    echo -e "${GREEN}✓ 옵션 상태: $STATUS${NC}"
    echo -e "${GREEN}✓ 지급액: \$$PAYOFF${NC}"
else
    PAYOFF=0
    STATUS="OTM (Out of The Money)"
    echo -e "${YELLOW}⚠️  옵션 상태: $STATUS${NC}"
    echo -e "${YELLOW}⚠️  지급액: \$0${NC}"
fi

# 6. 증명 생성 시뮬레이션
echo ""
echo -e "${YELLOW}[Step 6] BitVMX 증명 생성 (시뮬레이션)${NC}"

# 증명 데이터 생성
PROOF_DATA=$(cat << EOF
{
  "option_type": "$OPTION_TYPE",
  "strike_price": $STRIKE_PRICE,
  "spot_price": $SPOT_PRICE,
  "quantity": $QUANTITY,
  "payoff": $PAYOFF,
  "status": "$STATUS",
  "timestamp": $(date +%s)
}
EOF
)

echo "$PROOF_DATA" > option_settlement_proof.json
echo -e "${GREEN}✓ 증명 데이터 생성 완료${NC}"

# 7. Bitcoin Script 생성 (시뮬레이션)
echo ""
echo -e "${YELLOW}[Step 7] Bitcoin Script 생성${NC}"

# 간단한 Bitcoin Script 예시
BITCOIN_SCRIPT="OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG"
echo -e "${GREEN}✓ Settlement Script: $BITCOIN_SCRIPT${NC}"

# 8. 결과 요약
echo ""
echo "===================================="
echo -e "${GREEN}✅ PoC 실행 완료!${NC}"
echo "===================================="
echo ""
echo "📊 실행 결과:"
echo "  • Oracle 가격: \$$SPOT_PRICE"
echo "  • 옵션 타입: Call"
echo "  • 행사가: \$$STRIKE_PRICE" 
echo "  • 상태: $STATUS"
echo "  • 지급액: \$$PAYOFF"
echo ""
echo "📄 생성된 파일:"
echo "  • option_settlement_proof.json - 증명 데이터"
echo ""
echo "🔗 다음 단계:"
echo "  1. Bitcoin Testnet에 증명 앵커링"
echo "  2. 실제 BTC 정산 실행"
echo "  3. 웹 인터페이스 연동"
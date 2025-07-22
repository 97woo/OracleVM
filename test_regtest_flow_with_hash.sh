#!/bin/bash

# BTCFi Oracle VM - 해시 기반 앵커링 테스트
# 증명 데이터의 해시를 OP_RETURN에 저장

set -e

echo "====================================="
echo "🔐 BTCFi 해시 앵커링 테스트"
echo "====================================="

# 색상 정의
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# 1. 기존 POC 실행
echo -e "${YELLOW}[Step 1] 옵션 정산 실행${NC}"
./poc_demo.sh

# 2. 증명 데이터 해시 생성
echo ""
echo -e "${YELLOW}[Step 2] 증명 데이터 해시 생성${NC}"

# 전체 증명 데이터 읽기
PROOF_DATA=$(cat option_settlement_proof.json)
echo "원본 데이터:"
echo "$PROOF_DATA" | jq '.'

# SHA256 해시 생성
PROOF_HASH=$(echo -n "$PROOF_DATA" | sha256sum | cut -d' ' -f1)
echo -e "${GREEN}✓ 증명 해시 (SHA256): $PROOF_HASH${NC}"

# 3. 메타데이터 생성 (해시 + 타입)
echo ""
echo -e "${YELLOW}[Step 3] 앵커링 메타데이터 생성${NC}"

# 프로토콜 버전 (1바이트) + 데이터 타입 (1바이트) + 해시 (32바이트)
# 01 = 버전 1, 01 = 옵션 정산
METADATA="0101${PROOF_HASH}"
echo -e "${GREEN}✓ 메타데이터: ${METADATA:0:40}...${NC}"
echo "  • 버전: 01"
echo "  • 타입: 01 (옵션 정산)"
echo "  • 해시: $PROOF_HASH"

# 4. Bitcoin regtest 확인
echo ""
echo -e "${YELLOW}[Step 4] Bitcoin regtest 노드 확인${NC}"
if bitcoin-cli -regtest getblockchaininfo >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Bitcoin regtest 노드 실행 중${NC}"
else
    echo -e "${RED}✗ Bitcoin regtest 노드가 실행되지 않음${NC}"
    exit 1
fi

# 5. 트랜잭션 생성
echo ""
echo -e "${YELLOW}[Step 5] 해시 앵커링 트랜잭션 생성${NC}"

ADDRESS=$(bitcoin-cli -regtest getnewaddress "hash_anchor")
echo "앵커링 주소: $ADDRESS"

# UTXO 확인
UTXOS=$(bitcoin-cli -regtest listunspent | jq -r '.[0]')
if [ "$UTXOS" = "null" ]; then
    echo -e "${YELLOW}⚠️  UTXO가 없습니다. 블록 생성 중...${NC}"
    bitcoin-cli -regtest generatetoaddress 101 "$ADDRESS" >/dev/null
    UTXOS=$(bitcoin-cli -regtest listunspent | jq -r '.[0]')
fi

# 트랜잭션 생성
if [ "$UTXOS" != "null" ]; then
    TXID=$(echo $UTXOS | jq -r '.txid')
    VOUT=$(echo $UTXOS | jq -r '.vout')
    AMOUNT=$(echo $UTXOS | jq -r '.amount')
    SEND_AMOUNT=$(echo "$AMOUNT - 0.0001" | bc)
    
    # OP_RETURN에 메타데이터 포함
    RAW_TX=$(bitcoin-cli -regtest createrawtransaction \
        "[{\"txid\":\"$TXID\",\"vout\":$VOUT}]" \
        "[{\"$ADDRESS\":$SEND_AMOUNT},{\"data\":\"$METADATA\"}]")
    
    # 서명 및 전송
    SIGNED_TX=$(bitcoin-cli -regtest signrawtransactionwithwallet "$RAW_TX" | jq -r '.hex')
    ANCHOR_TXID=$(bitcoin-cli -regtest sendrawtransaction "$SIGNED_TX")
    
    echo -e "${GREEN}✓ 해시 앵커링 완료: $ANCHOR_TXID${NC}"
    
    # 블록 생성
    bitcoin-cli -regtest generatetoaddress 1 "$ADDRESS" >/dev/null
    
    # 트랜잭션 확인
    echo ""
    echo -e "${YELLOW}[Step 6] 앵커링 검증${NC}"
    TX_DATA=$(bitcoin-cli -regtest getrawtransaction "$ANCHOR_TXID" true | jq '.vout[] | select(.scriptPubKey.type == "nulldata")')
    echo "$TX_DATA" | jq '.'
    
    # 저장된 데이터 디코드
    STORED_HEX=$(echo "$TX_DATA" | jq -r '.scriptPubKey.hex' | cut -c5-)
    echo ""
    echo "저장된 데이터 분석:"
    echo "  • 버전: ${STORED_HEX:0:2}"
    echo "  • 타입: ${STORED_HEX:2:2}"
    echo "  • 해시: ${STORED_HEX:4:64}"
fi

# 7. 오프체인 저장소 시뮬레이션
echo ""
echo -e "${YELLOW}[Step 7] 오프체인 증명 저장${NC}"

# IPFS 해시 시뮬레이션 (실제로는 IPFS에 업로드)
IPFS_HASH="QmXxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
echo "원본 증명 데이터를 오프체인에 저장:"
echo "  • IPFS: $IPFS_HASH"
echo "  • 로컬: option_settlement_proof.json"

# 검증 정보 저장
cat > proof_verification_info.json << EOF
{
  "anchor_txid": "$ANCHOR_TXID",
  "proof_hash": "$PROOF_HASH",
  "ipfs_hash": "$IPFS_HASH",
  "local_file": "option_settlement_proof.json",
  "timestamp": $(date +%s)
}
EOF

echo -e "${GREEN}✓ 검증 정보 저장: proof_verification_info.json${NC}"

echo ""
echo "====================================="
echo -e "${GREEN}✅ 해시 앵커링 테스트 완료!${NC}"
echo "====================================="
echo ""
echo "📊 결과 요약:"
echo "  • 증명 해시: $PROOF_HASH"
echo "  • 앵커 TX: $ANCHOR_TXID"
echo "  • 온체인: 34바이트 (버전+타입+해시)"
echo "  • 오프체인: 전체 증명 데이터"
echo ""
echo "💡 장점:"
echo "  • 더 많은 데이터 저장 가능"
echo "  • 증명 무결성 보장"
echo "  • 확장 가능한 구조"
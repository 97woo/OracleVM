# Bitcoin Testnet 옵션 테스트 가이드

## 🚀 소개

이 가이드는 Bitcoin Testnet에서 BitVMX 기반 단방향 옵션을 테스트하는 방법을 설명합니다.

## 📝 사전 준비

1. Bitcoin Core 또는 Testnet 지원 지갑 설치
2. Testnet BTC 획득 (faucet 사용)
3. Rust 및 프로젝트 빌드

## 🔑 1단계: 테스트 키 생성

```bash
cargo run --bin testnet-deploy -- generate-keys
```

출력 예시:
```
🔑 Testnet 테스트 키 생성:

[구매자]
  비밀키: d8a1e1224e63135765bde9dc8a2c8e403eee8be73d3589d58c5ddbf9dce3fdf4
  공개키: 03bf751f0d2d22e6f0163c9acaa14ae04e6e1a004cb4a24d893c1f86314e79d5de
  주소: tb1qt8rdur557nz338g3lekc6458pj0dl63c0s9904

[판매자]
  비밀키: 143c9cf988b64adb053a0ee3ef7e3bfb5a3c424e0112c606df4d158ef0e59f2f
  공개키: 022f09529804cb607b868d3446cf48fde456573a55a19f9a01752f7dd4e31c36d9
  주소: tb1q3cxk9w30k2fa7t7cxa79jqvrvj96y6qayrh5as

[검증자]
  비밀키: 3e3d98605246602c99a7b29f251f1b7a761c398dec3ebbee7cba2a4827a710ef
  공개키: 02bd4602772c3e6bb855682def9ec6ec3a26fc36777a5598b119b4eca3f38f83bd
  주소: tb1qyewluv8r09lc3yrvdw3nccng0z7e9e9526gc6v
```

## 💵 2단계: Testnet BTC 획듍

위에서 생성된 주소로 Testnet BTC를 받아야 합니다:

### Faucet 사이트:
- https://coinfaucet.eu/en/btc-testnet/
- https://testnet-faucet.mempool.co/
- https://bitcoinfaucet.uo1.net/

### 필요 금액:
- 구매자: 0.02 tBTC (프리미엄 + 수수료)
- 판매자: 0.11 tBTC (담보 + 수수료)

## 📝 3단계: 옵션 컨트랙트 주소 생성

```bash
cargo run --bin testnet-deploy -- create-option-address \
  --buyer-pubkey 03bf751f0d2d22e6f0163c9acaa14ae04e6e1a004cb4a24d893c1f86314e79d5de \
  --seller-pubkey 022f09529804cb607b868d3446cf48fde456573a55a19f9a01752f7dd4e31c36d9 \
  --verifier-pubkey 02bd4602772c3e6bb855682def9ec6ec3a26fc36777a5598b119b4eca3f38f83bd \
  --strike 50000 \
  --expiry 2580000
```

출력:
```
📝 옵션 컨트랙트 Taproot 주소:
tb1pha637rfdytn0q93unt92zjhqfehp5qzvkj3ymzfur7rrznne6h0qkjvnu3
```

## 🚀 4단계: 옵션 활성화

### 4.1 트랜잭션 생성

이 단계는 수동으로 진행해야 합니다:

1. **구매자**: 0.01 tBTC (프리미엄)를 옵션 주소로 전송
2. **판매자**: 0.1 tBTC (담보)를 옵션 주소로 전송

### 4.2 트랜잭션 확인

Testnet Explorer에서 확인:
- https://mempool.space/testnet
- https://blockstream.info/testnet

## 📊 5단계: 만기 시뮬레이션

현재 블록 높이 확인:
```bash
bitcoin-cli -testnet getblockcount
```

만기 블록(2,580,000)에 도달하면:

### ITM (In The Money) 시나리오
BTC 가격이 $52,000이면 구매자가 옵션 행사

### OTM (Out of The Money) 시나리오  
BTC 가격이 $48,000이면 판매자가 담보 회수

## 🔍 6단계: 정산 트랜잭션 생성

정산은 BitVMX를 통해 자동으로 이루어집니다:

1. Oracle이 만기 시점 BTC 가격 수집
2. BitVMX가 정산 금액 계산 및 증명 생성
3. 검증자가 증명 확인 및 서명
4. Bitcoin Script가 자동으로 정산 실행

## 👀 예상 결과

### Call ITM 예시 (Spot: $52,000)
- 구매자: 0.11 tBTC 수령 (프리미엄 + 담보)
- 판매자: 0 tBTC
- 수익률: 1,000%

### Call OTM 예시 (Spot: $48,000)
- 구매자: 0 tBTC (프리미엄 손실)
- 판매자: 0.11 tBTC 유지 (프리미엄 + 담보)
- 손실: -100%

## ⚠️ 주의사항

1. **테스트넷 전용**: 이 코드는 Bitcoin Testnet에서만 작동합니다
2. **비밀키 보관**: 생성된 비밀키를 안전하게 보관하세요
3. **만기 시간**: 설정된 블록 높이에 도달할 때까지 기다려야 합니다
4. **가스 비용**: Testnet에서도 트랜잭션 수수료가 필요합니다

## 🛠️ 문제 해결

### 트랜잭션이 확인되지 않음
- 네트워크 혼잡도 확인
- 수수료가 충분한지 확인
- Testnet 노드 연결 상태 확인

### 옵션 주소가 잘못됨
- 공개키가 올바른지 확인
- 행사가와 만기 블록이 올바른지 확인

## 📚 참고 자료

- [Bitcoin Testnet Wiki](https://en.bitcoin.it/wiki/Testnet)
- [Taproot BIP341](https://github.com/bitcoin/bips/blob/master/bip-0341.mediawiki)
- [BitVMX Documentation](https://github.com/FairgateLabs/BitVMX)

---

**🎆 Bitcoin L1 네이티브 옵션의 미래를 함께 테스트해보세요!**
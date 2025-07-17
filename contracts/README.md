# 지갑/컨트랙트 모듈 + POOL 관리 모듈

## 개요
Bitcoin Layer 1에서 동작하는 온체인 모듈로, 사용자 지갑 관리, 옵션 컨트랙트 생성/관리, 유동성 풀 상태 관리를 담당합니다.

## 주요 기능

### 1. 지갑 관리
- **Taproot 지갑**: 사용자 개인키 관리
- **멀티시그 지갑**: 보안 강화를 위한 다중 서명
- **자금 입출금**: BTC 입금/출금 처리

### 2. 옵션 컨트랙트 관리
- **컨트랙트 생성**: BitVMX와 연동한 옵션 컨트랙트 생성
- **포지션 추적**: 사용자별 옵션 포지션 관리
- **만료 처리**: 자동 정산 및 청산

### 3. 유동성 풀 관리
- **풀 상태 추적**: 총 유동성, 델타 포지션 관리
- **리스크 관리**: 포지션 한도 및 마진 관리
- **수수료 관리**: 거래 수수료 및 LP 보상

## 기술 스택

### Bitcoin Script + Taproot
```bitcoin
OP_CHECKSIG OP_IF
  <BitVMX_proof_verification>
OP_ELSE
  <timelock> OP_CHECKLOCKTIMEVERIFY OP_DROP
  <fallback_pubkey> OP_CHECKSIG
OP_ENDIF
```

### 상태 관리 구조
```rust
pub struct PoolState {
    pub total_liquidity: u64,       // 총 유동성 (sats)
    pub total_call_delta: f64,      // 총 Call 델타
    pub total_put_delta: f64,       // 총 Put 델타
    pub active_positions: Vec<Position>,
    pub pending_settlements: Vec<Settlement>,
}

pub struct Position {
    pub user_pubkey: [u8; 32],
    pub option_type: OptionType,
    pub strike: u64,
    pub quantity: u64,
    pub expiry: u64,
    pub premium_paid: u64,
}
```

## 인터페이스

### BitVMX 모듈과의 연동
```rust
// 거래 발생 시: Contract → BitVMX
pub fn create_option_contract(
    option_params: OptionParams,
) -> Result<BitVMXContract, Error>;

// 만기 시: BitVMX → Contract
pub fn settle_option(
    settlement_proof: BitVMXProof,
    payout_amount: u64,
) -> Result<Transaction, Error>;
```

### 계산 모듈과의 연동
```rust
// 풀 상태 갱신: Contract → Calculation
pub fn update_pool_state(
    new_position: Position,
) -> Result<(), Error>;

// 델타 조회: Calculation ← Contract
pub fn get_current_delta() -> f64;
```

## 구현 우선순위

1. **기본 지갑 기능** - Taproot 지갑 생성 및 관리
2. **간단한 옵션 컨트랙트** - 기본적인 Call/Put 컨트랙트
3. **풀 상태 관리** - 델타 추적 및 리스크 관리
4. **BitVMX 통합** - 증명 검증 및 자동 정산
5. **고급 기능** - 복잡한 옵션 전략 지원

## 보안 고려사항

- **개인키 보안**: 하드웨어 지갑 연동
- **스마트 컨트랙트 보안**: 코드 감사 및 테스트
- **자금 보안**: 멀티시그 및 타임락 활용
- **오라클 보안**: 가격 조작 방지

## 테스트넷 배포

1. **Bitcoin Testnet**: 초기 테스트
2. **Bitcoin Signet**: 고급 기능 테스트
3. **Bitcoin Mainnet**: 프로덕션 배포

## 미구현 상태
현재 설계 단계이며, 구체적인 구현이 필요합니다.
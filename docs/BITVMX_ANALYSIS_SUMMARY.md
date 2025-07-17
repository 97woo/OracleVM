# BitVMX Oracle VM 프로젝트 분석 요약

## 🎯 프로젝트 개요
- **목표**: Bitcoin L1에서 Oracle 기반 옵션 정산 시스템 구현
- **핵심 기술**: BitVMX CPU를 활용한 검증 가능한 오프체인 계산
- **아키텍처**: Multi-exchange Oracle → Aggregator → BitVMX → Bitcoin L1 앵커링

## 🏗 시스템 구조

### Oracle Layer (완료됨)
```
Oracle Node (Binance/Coinbase/Kraken) 
→ gRPC Aggregator (2/3 합의)
→ 집계된 가격 데이터
```

### BitVMX Integration (구현 필요)
```
집계된 가격 → BitVMX CPU (옵션 정산) → 검증 가능한 증명 생성
```

### Bitcoin L1 Anchoring
```
OP_RETURN: 결과 해시 + 메타데이터 (80바이트)
Taproot Vault: 담보 Lock-up 및 조건부 해제
Bitcoin Script: 분쟁 시 전체 검증 (1000+바이트)
```

## 🔧 BitVMX 핵심 이해

### 동작 원리
1. **오프체인 계산**: 복잡한 옵션 정산을 RISC-V VM에서 실행
2. **Trace 생성**: 모든 실행 단계를 TraceRead/TraceStep으로 기록
3. **Bitcoin Script 변환**: 각 단계를 검증 가능한 Script로 자동 변환
4. **챌린지-응답**: 분쟁 시에만 온체인 검증 실행

### 핵심 통찰
- BitVMX는 Bitcoin Script에서 직접 계산하지 않음
- 오프체인 계산 결과를 단순한 비교 연산으로 검증
- 복잡한 연산을 기본 연산들로 분해하여 각각 검증
- 실제로는 "챌린지" 용도로만 사용됨 (99.9%는 오프체인만)

## 📁 수정 필요한 BitVMX 파일들

### Level 1: 필수 수정 (80% 시간)
1. **fetcher.rs**: 새로운 옵션 명령어 추가
   ```rust
   LoadOraclePrice(x) => op_load_oracle_price(&x, program),
   SettleCallOption(x) => op_settle_call_option(&x, program),
   VerifyMerkleProof(x) => op_verify_merkle_proof(&x, program),
   ```

2. **program.rs**: Oracle 섹션 추가
   ```rust
   const ORACLE_BASE_ADDRESS: u32 = 0xD000_0000;
   impl Program {
       pub fn add_oracle_section(&mut self, oracle_data: OracleData) { ... }
   }
   ```

3. **constants.rs**: 주소 상수 정의

### Level 2: 새 함수 구현 (15% 시간)
```rust
pub fn op_load_oracle_price() -> (TraceRead, TraceRead, TraceWrite, MemoryWitness)
pub fn op_settle_call_option() -> (TraceRead, TraceRead, TraceWrite, MemoryWitness)  
pub fn op_verify_merkle_proof() -> (TraceRead, TraceRead, TraceWrite, MemoryWitness)
```

## 🔒 Lock-up 시스템

### Taproot Vault 구조
```rust
- 담보 보관: Taproot 다중 조건부 스크립트
- 정상 정산: Oracle 검증 + 정산 증명 → 자동 해제
- 만료 환불: 시간 경과 + 판매자 서명
- 비상 환불: 양쪽 서명
- 분쟁 해결: BitVMX 챌린지-응답
```

## 💡 핵심 혁신 포인트

### 튜링 완전성 달성
- BitVMX를 통해 Bitcoin이 사실상 튜링 완전해짐
- 모든 계산을 기본 연산으로 분해 → 각각 검증
- "계산은 자유롭게, 검증은 Bitcoin에서"

### 챌린지 중심 설계
- 일상 사용: 오프체인만 (빠름, 저렴)
- 분쟁 발생: 온체인 검증 (안전, 확실)  
- 사기 억제: 경제적 처벌 메커니즘

## 📊 데이터 흐름

### Input
```rust
struct OracleData {
    btc_price: u32,           // 45,123
    timestamp: u64,
    merkle_root: [u8; 32],
    proof: Vec<[u8; 32]>,
}

struct OptionContract {
    strike_price: u32,        // 40,000
    option_type: OptionType,  // Call/Put
    expiry_timestamp: u64,
    collateral_amount: u64,
}
```

### Output
```rust
struct SettlementResult {
    payoff_amount: u64,       // 5,123 USD → satoshi
    is_valid: bool,
    execution_step: u64,
    final_hash: [u8; 20],
}

struct BitcoinProof {
    verification_script: Vec<u8>,  // Bitcoin Script
    witness_data: Vec<Vec<u8>>,
    execution_trace: Vec<TraceRWStep>,
}
```

## 🎯 실제 구현 전략

### 복사-수정 패턴
- 기존 ADD 명령어 코드를 복사
- 덧셈 로직만 옵션 정산 로직으로 변경
- TraceRead/TraceWrite 패턴은 동일하게 유지
- Bitcoin Script는 BitVMX가 자동 생성

### 테스트 접근
- main.rs의 ADDI 예제를 옵션 정산으로 변경
- 단순한 케이스부터 검증 (Call 옵션, 단일 가격)
- 점진적으로 복잡도 증가

## 🚀 비즈니스 임팩트

### 기술적 혁신
- Bitcoin DeFi 최초 구현
- 완전 탈중앙화 옵션 거래
- 수학적 증명 기반 투명성

### 시장 기회  
- Bitcoin 홀더들의 DeFi 참여 가능
- 다른 체인 대비 보안 우위
- 새로운 금융 상품 카테고리 창조

## 📋 현재 상태
- Oracle 수집/집계: ✅ 완료
- BitVMX 통합: 🚧 진행 중
- Lock-up 시스템: ⏳ 설계 단계
- 테스트넷 배포: ⏳ 계획 단계
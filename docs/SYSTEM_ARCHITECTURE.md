# BTCFi Option Settlement System Architecture

## 모듈 구조

### 1. 오라클 모듈 (Oracle Module) - 오프체인
**위치**: `crates/oracle-node/`
**역할**: 
- 다중 거래소 가격 데이터 수집 (Binance, Coinbase, Kraken)
- 2/3 컨센서스 가격 검증
- gRPC를 통한 가격 데이터 전송

**구현 파일**:
- `src/main.rs` - Oracle Node 메인
- `src/grpc_client.rs` - Aggregator 통신
- `src/binance.rs`, `src/coinbase.rs`, `src/kraken.rs` - 거래소 클라이언트

### 2. 지갑/컨트랙트 모듈 + POOL 관리 모듈 - 온체인
**위치**: `contracts/` (미구현)
**역할**:
- 사용자 지갑 관리
- 옵션 컨트랙트 생성/관리
- 유동성 풀 상태 관리
- 자금 입출금 처리

**필요 구현**:
- Bitcoin Script/Taproot 컨트랙트
- 풀 상태 추적
- 사용자 포지션 관리

### 3. BitVMX 모듈 - 오프체인
**위치**: `bitvmx_protocol/`
**역할**:
- 옵션 정산 로직 실행
- 증명 생성 및 검증
- 온체인 앵커링

**구현 파일**:
- `src/oracle_bridge.rs` - Oracle → BitVMX 데이터 변환
- `src/settlement_executor.rs` - 정산 실행
- `BitVMX-CPU/docker-riscv32/src/hello-world.c` - RISC-V 정산 로직

### 4. 계산 모듈 (only GET) - 오프체인
**위치**: `calculation/` (미구현)
**역할**:
- 프리미엄 계산 (Black-Scholes 등)
- 델타/감마 등 그릭스 계산
- 풀 상태 분석

**필요 구현**:
- JSON API 서버
- 실시간 계산 엔진
- 프론트엔드 연동

## 시스템 플로우

### Update 사이클
```
1번(Oracle) → 4번(Calculation)
- 오라클 가격 확인
- 프리미엄 재계산
- 프론트엔드 데이터 갱신
```

### 거래 발생 시
```
1. 2번(Contract) → 3번(BitVMX): 컨트랙트 생성
2. 2번(Contract) → 4번(Calculation): 풀 상태 갱신
3. Update 사이클 실행
```

### 만기 시
```
1. 1번(Oracle) → 3번(BitVMX): 오라클 데이터로 청산 금액 계산
2. 3번(BitVMX) → 2번(Contract): 청산 금액 유저에게 전송
3. 2번(Contract) → 4번(Calculation): 풀 상태 갱신
4. Update 사이클 실행
```

## 현재 구현 상태

✅ **완료**:
- 1번 오라클 모듈 (가격 수집, 컨센서스)
- 3번 BitVMX 모듈 (정산 로직, 데이터 변환)
- 4번 계산 모듈 (Black-Scholes 기반 API 서버)

🚧 **부분 구현**:
- 2번 지갑/컨트랙트 모듈 (Taproot 기반 옵션 컨트랙트 핵심 로직 구현 완료)

## 다음 단계 우선순위

1. **지갑/컨트랙트 모듈 기능 완성** (트랜잭션 생성 등)
2. **전체 시스템 통합 테스트**
3. **프론트엔드 연동**
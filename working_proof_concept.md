# BitVMX 실제 증명 생성 방법

## 현재 상황

1. **작동하는 것**:
   - BitVMX Prover/Verifier Docker 컨테이너 (부분적)
   - BitVMX-CPU 에뮬레이터 (빌드 완료)
   - 옵션 정산 로직 (C/Rust)

2. **문제점**:
   - RISC-V 32비트 컴파일러 부재
   - BitVMX-CPU 서브모듈 비어있음
   - ELF 파일 형식 호환성 문제

## 실제 증명 생성 단계

### 1. RISC-V 프로그램 준비
```bash
# RISC-V 32비트 툴체인 설치 (Ubuntu/Debian)
sudo apt-get install gcc-riscv64-linux-gnu

# 크로스 컴파일
riscv64-linux-gnu-gcc -march=rv32i -mabi=ilp32 \
    -nostdlib -nostartfiles \
    option_settlement.c -o option_settlement.elf
```

### 2. BitVMX 에뮬레이터 실행
```bash
# 실행 추적 생성
./emulator execute \
    --elf option_settlement.elf \
    --input "00000000404b4c0080584f0064000000" \
    --trace > execution_trace.json
```

### 3. Merkle Commitment 생성
```bash
# ROM commitment
./emulator generate-rom-commitment --elf option_settlement.elf

# Instruction mapping
./emulator instruction-mapping > instruction_mapping.txt
```

### 4. Bitcoin Script 생성
- 실행 추적을 Bitcoin Script로 변환
- Challenge-response 프로토콜 구현
- Winternitz 서명 생성

## 대안: 간소화된 증명 시스템

현재 즉시 구현 가능한 방법:

1. **Merkle Tree 기반 증명**
   - 옵션 정산 결과를 Merkle Tree로 구성
   - Root hash만 온체인 저장
   - 오프체인에서 증명 검증

2. **시간 잠금 계약**
   - 미리 서명된 트랜잭션 사용
   - 만기 시 자동 실행
   - 분쟁 시 challenge period

3. **신뢰할 수 있는 오라클**
   - 다중 서명 방식
   - 평판 시스템
   - 슬래싱 메커니즘

## 결론

BitVMX는 강력한 시스템이지만, 완전한 구현을 위해서는:
- 전체 소스 코드 접근
- 적절한 개발 환경
- 상세한 문서

현재는 MVP를 위해 간소화된 방식을 사용하고,
BitVMX가 완전히 공개되면 통합하는 것이 현실적입니다.
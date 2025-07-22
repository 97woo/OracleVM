# BitVMX 옵션 정산 증명 생성 가이드

## 개요
BitVMX를 사용하여 Bitcoin 옵션 정산의 온체인 검증 가능한 증명을 생성하는 방법입니다.

## 사전 준비

### 1. RISC-V 툴체인 설치 (macOS)
```bash
# Homebrew로 설치
brew tap riscv-software-src/riscv
brew install riscv-gnu-toolchain

# 설치 확인
riscv64-unknown-elf-gcc --version
```

### 2. 프로젝트 클론
```bash
git clone https://github.com/97woo/OracleVM.git
cd OracleVM
git checkout dev
git submodule update --init --recursive
```

## 증명 생성 단계

### Step 1: C 코드를 RISC-V 바이너리로 컴파일

```bash
cd bitvmx_protocol/BitVMX-CPU/docker-riscv32/src

# 필요한 의존성 다운로드 (처음 한 번만)
curl -LJO https://github.com/gcc-mirror/gcc/raw/master/libgcc/config/riscv/riscv-asm.h
curl -LJO https://github.com/gcc-mirror/gcc/raw/master/libgcc/config/epiphany/mulsi3.c
curl -LJO https://github.com/gcc-mirror/gcc/raw/master/libgcc/config/riscv/div.S

# RISC-V ELF 컴파일
riscv64-unknown-elf-gcc -march=rv32im -mabi=ilp32 -nostdlib \
  -T../linker/link.ld entrypoint.s option_settlement.c div.S mulsi3.c \
  -o option_settlement.elf

# 컴파일 확인
file option_settlement.elf
# 출력: ELF 32-bit LSB executable, UCB RISC-V...
```

### Step 2: BitVMX 에뮬레이터로 옵션 정산 실행

프로젝트 루트로 이동:
```bash
cd /path/to/OracleVM
```

#### 테스트 케이스 1: Call Option ITM
```bash
# Strike: $50,000, Spot: $52,000, Quantity: 1.0
cargo run --release -p emulator execute \
  --elf bitvmx_protocol/BitVMX-CPU/docker-riscv32/src/option_settlement.elf \
  --input 00000000404b4c0080584f0064000000 \
  --stdout
```

예상 출력:
```
=== BTCFi Option Settlement ===
Option Type: CALL
Status: Call ITM
Payout: Positive
Settlement completed successfully
```

#### 테스트 케이스 2: Put Option ITM
```bash
# Strike: $50,000, Spot: $48,000, Quantity: 2.0
cargo run --release -p emulator execute \
  --elf bitvmx_protocol/BitVMX-CPU/docker-riscv32/src/option_settlement.elf \
  --input 01000000404b4c00003e4900c8000000 \
  --stdout
```

### Step 3: 실행 트레이스 생성

```bash
# 트레이스 파일 생성
cargo run --release -p emulator execute \
  --elf bitvmx_protocol/BitVMX-CPU/docker-riscv32/src/option_settlement.elf \
  --input 00000000404b4c0080584f0064000000 \
  --trace --stdout > option_execution_output.txt

# 트레이스 확인 (약 2,247 라인)
wc -l option_execution_output.txt
```

### Step 4: Bitcoin Script 증명 생성

```bash
cd bitvmx_protocol
python3 trace_to_script.py
```

출력:
```
=== BitVMX 트레이스 → Bitcoin Script 변환 ===

트레이스 라인 수: 2247
생성된 Bitcoin Script:
...
✅ Bitcoin Script 저장됨: option_settlement_proof.script
```

## 생성된 증명 구조

### 1. **option_settlement.elf** (13,936 bytes)
- RISC-V 32비트 실행 파일
- 옵션 정산 로직 포함

### 2. **option_execution_output.txt**
- 2,247개의 실행 단계
- 각 단계별 상태와 해시값

### 3. **option_settlement_proof.script**
- 36줄의 Bitcoin Script
- 온체인 검증 가능한 증명

## 입력 데이터 형식

```c
typedef struct {
    uint32_t option_type;    // 0 = Call, 1 = Put
    uint32_t strike_price;   // 행사가 (USD * 100)
    uint32_t spot_price;     // 현재가 (USD * 100)
    uint32_t quantity;       // 수량 (unit * 100)
} OptionInput;
```

### 입력 예시
- Call ITM: `00000000404b4c0080584f0064000000`
  - Type: 0 (Call)
  - Strike: $50,000
  - Spot: $52,000
  - Quantity: 1.0

## 증명 검증

생성된 Script의 주요 구성:
```
1. Program Commitment (프로그램 해시)
2. Option Type Verification (옵션 타입 검증)
3. ITM/OTM Decision (행사 여부 판단)
4. Final Settlement (최종 정산 검증)
```

## 온체인 앵커링 (다음 단계)

생성된 증명을 Bitcoin 네트워크에 앵커링:

```python
# 예시 코드
from contracts.bitvmx_presign import BitVMXPreSigner

presigner = BitVMXPreSigner()
tx = presigner.create_settlement_transaction(
    option_utxo,
    settlement_script,
    expiry_height
)
```

## 문제 해결

1. **컴파일 에러**: 
   - `riscv-asm.h` 파일이 없다면 위의 curl 명령어 재실행

2. **에뮬레이터 실행 에러**:
   - ELF 파일 경로 확인
   - 입력 hex 길이가 32자인지 확인

3. **트레이스 변환 에러**:
   - `option_execution_output.txt` 파일 존재 확인
   - Python 3.6+ 설치 확인

## 참고 자료

- 옵션 정산 로직: `/bitvmx_protocol/BitVMX-CPU/docker-riscv32/src/option_settlement.c`
- 입력 생성 헬퍼: `/bitvmx_protocol/option_settlement/src/main.rs`
- BitVMX 에뮬레이터: `/bitvmx_protocol/BitVMX-CPU/emulator/`

---

작성일: 2025-07-22
작성자: BTCFi Team
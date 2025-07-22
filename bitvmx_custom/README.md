# BitVMX 옵션 정산 커스텀 구현

우리가 직접 구현한 BitVMX 옵션 정산 로직입니다.

## 파일 설명

- `option_settlement.c` - 옵션 정산 C 코드
- `compile_option.sh` - RISC-V 컴파일 스크립트
- `link.ld` - 링커 스크립트 (필요시 추가)

## 사용법

1. RISC-V 툴체인 설치:
```bash
brew install riscv-gnu-toolchain
```

2. 컴파일:
```bash
./compile_option.sh
```

3. BitVMX 에뮬레이터로 실행:
```bash
cargo run --release -p emulator execute --elf option_settlement.elf --input <hex_input>
```

## 입력 형식

16바이트 입력 (hex):
- 0-3: option_type (0=Call, 1=Put)
- 4-7: strike_price (USD * 100)
- 8-11: spot_price (USD * 100)
- 12-15: quantity (unit * 100)

예시:
- Call ITM: `00000000404b4c0080584f0064000000`
  - Type: 0 (Call)
  - Strike: $50,000
  - Spot: $52,000
  - Quantity: 1.0
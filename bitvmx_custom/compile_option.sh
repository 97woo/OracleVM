#!/bin/bash

# RISC-V 32비트용 옵션 정산 컴파일 스크립트

echo "옵션 정산 프로그램 컴파일 중..."

# RISC-V 툴체인 확인
if ! command -v riscv32-unknown-elf-gcc &> /dev/null; then
    echo "Error: RISC-V 툴체인이 설치되지 않았습니다."
    echo "설치: brew install riscv-gnu-toolchain"
    exit 1
fi

# 컴파일
riscv32-unknown-elf-gcc \
    -march=rv32i \
    -mabi=ilp32 \
    -nostdlib \
    -nostartfiles \
    -T link.ld \
    -o option_settlement.elf \
    option_settlement.c

echo "컴파일 완료: option_settlement.elf"

# 16진수 덤프 생성 (검증용)
riscv32-unknown-elf-objdump -d option_settlement.elf > option_settlement.dump

echo "디스어셈블리 생성: option_settlement.dump"
#!/usr/bin/env python3
"""
실제 BitVMX 실행 추적 생성
BitVMX 프로토콜 라이브러리를 직접 사용
"""

import sys
import os
import json
import subprocess
import tempfile
from pathlib import Path

# BitVMX 프로토콜 라이브러리 경로 추가
sys.path.append(os.path.join(os.path.dirname(__file__), 'bitvmx_protocol'))

def generate_execution_trace(elf_file: str, input_hex: str) -> dict:
    """BitVMX 에뮬레이터로 실행 추적 생성"""
    
    # emulator 실행 파일 찾기
    emulator_paths = [
        "./bitvmx_protocol/BitVMX-CPU/emulator/target/release/emulator",
        "./bitvmx_protocol/BitVMX-CPU/emulator/target/debug/emulator",
        "emulator"  # PATH에 있는 경우
    ]
    
    emulator = None
    for path in emulator_paths:
        if os.path.exists(path) or subprocess.run(["which", path], capture_output=True).returncode == 0:
            emulator = path
            break
    
    if not emulator:
        print("⚠️  BitVMX 에뮬레이터를 찾을 수 없습니다.")
        print("   Cargo로 빌드를 시도합니다...")
        
        # Cargo 빌드 시도
        bitvmx_cpu_path = "./bitvmx_protocol/BitVMX-CPU"
        if os.path.exists(bitvmx_cpu_path):
            print(f"   Building in {bitvmx_cpu_path}...")
            result = subprocess.run(
                ["cargo", "build", "--release", "-p", "emulator"],
                cwd=bitvmx_cpu_path,
                capture_output=True,
                text=True
            )
            if result.returncode == 0:
                emulator = f"{bitvmx_cpu_path}/emulator/target/release/emulator"
            else:
                print(f"   빌드 실패: {result.stderr}")
        
    if not emulator or not os.path.exists(emulator):
        # 실행 추적 시뮬레이션 (실제가 아님)
        print("❌ 에뮬레이터 없음. 실제 실행 추적을 생성할 수 없습니다.")
        return None
    
    # 실제 에뮬레이터 실행
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as trace_file:
        trace_path = trace_file.name
    
    cmd = [
        emulator,
        "execute",
        "--elf", elf_file,
        "--input", input_hex,
        "--trace", trace_path
    ]
    
    print(f"실행 명령: {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode != 0:
        print(f"❌ 실행 실패: {result.stderr}")
        return None
    
    # 실행 추적 읽기
    with open(trace_path, 'r') as f:
        trace = json.load(f)
    
    os.unlink(trace_path)
    return trace


def main():
    print("=" * 50)
    print("🔧 실제 BitVMX 실행 추적 생성")
    print("=" * 50)
    
    # 테스트 입력
    option_type = 0  # Call
    strike_price = 50000 * 100  # $50,000 in cents
    spot_price = 52000 * 100    # $52,000 in cents
    quantity = 100  # 1.0 BTC
    
    # 16진수 입력 생성 (little-endian)
    def to_hex(n):
        return f"{n:08x}"[::-1].encode().hex()[:8]
    
    input_hex = to_hex(option_type) + to_hex(strike_price) + to_hex(spot_price) + to_hex(quantity)
    
    print(f"\n입력 데이터:")
    print(f"  • Type: Call")
    print(f"  • Strike: ${strike_price//100}")
    print(f"  • Spot: ${spot_price//100}")
    print(f"  • Quantity: {quantity/100} BTC")
    print(f"  • Hex: {input_hex}")
    
    # ELF 파일 경로
    elf_file = "./riscv_option/option_settlement.elf"
    if not os.path.exists(elf_file):
        elf_file = "./bitvmx_protocol/execution_files/test_input.elf"
    
    if not os.path.exists(elf_file):
        print(f"\n❌ ELF 파일을 찾을 수 없습니다: {elf_file}")
        return
    
    print(f"\nELF 파일: {elf_file}")
    
    # 실행 추적 생성
    print("\n실행 추적 생성 중...")
    trace = generate_execution_trace(elf_file, input_hex)
    
    if trace:
        print(f"\n✅ 실행 추적 생성 성공!")
        print(f"   • 총 스텝: {len(trace.get('steps', []))}")
        print(f"   • 메모리 접근: {len(trace.get('memory_accesses', []))}")
        
        # 추적 저장
        with open("bitvmx_execution_trace.json", "w") as f:
            json.dump(trace, f, indent=2)
        
        print(f"\n💾 실행 추적 저장: bitvmx_execution_trace.json")
    else:
        print("\n❌ 실행 추적 생성 실패")
        print("   BitVMX-CPU 에뮬레이터가 필요합니다.")
        print("   Docker 빌드가 완료되기를 기다려주세요.")


if __name__ == "__main__":
    main()
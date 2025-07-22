#!/usr/bin/env python3
"""
BitVMX 증명 생성 스크립트
"""

import subprocess
import json
import hashlib
import binascii

def generate_option_settlement_proof(
    option_type,
    strike_price_usd,
    spot_price_usd,
    quantity
):
    """옵션 정산을 위한 BitVMX 증명 생성"""
    
    # 1. 입력 데이터 준비 (USD를 cents로 변환)
    strike_cents = int(strike_price_usd * 100)
    spot_cents = int(spot_price_usd * 100)
    quantity_scaled = int(quantity * 100)
    
    # 각 값을 리틀 엔디안 형식으로 변환
    import struct
    input_bytes = struct.pack('<IIII', option_type, strike_cents, spot_cents, quantity_scaled)
    input_hex = input_bytes.hex()
    print(f"입력 데이터: {input_hex}")
    print(f"  옵션타입: {option_type}, 행사가: ${strike_price_usd}, 현물가: ${spot_price_usd}, 수량: {quantity}")
    
    # 2. BitVMX 에뮬레이터 실행
    elf_path = "./option_settlement_bitvmx_complete.elf"
    emulator_path = "./BitVMX-CPU/target/release/emulator"
    
    # 실행 트레이스 생성
    trace_cmd = [
        emulator_path, "execute",
        "--elf", elf_path,
        "--input", input_hex,
        "--trace"
    ]
    
    print("BitVMX 실행 중...")
    result = subprocess.run(trace_cmd, capture_output=True, text=True)
    
    if result.returncode != 0:
        print(f"에러: {result.stderr}")
        return None
    
    # 디버깅: 전체 출력 확인
    print(f"STDERR: {result.stderr[:200]}")
    print(f"STDOUT 첫 줄: {result.stdout.split(chr(10))[0] if result.stdout else 'empty'}")
    
    # 3. 실행 결과 파싱
    # stdout에서 결과 찾기
    lines = result.stdout.strip().split('\n')
    execution_result = None
    
    for line in lines:
        if "Execution result: Halt" in line:
            # Halt(정산금액, 스텝수) 형식
            parts = line.split("Halt(")[1].split(",")
            payout = int(parts[0])
            steps = int(parts[1].strip().rstrip(")"))
            execution_result = {
                "payout": payout,
                "steps": steps
            }
            break
    
    if not execution_result:
        print("실행 결과를 찾을 수 없습니다")
        return None
    
    print(f"정산 결과: ${payout/100:.2f} (스텝: {steps})")
    
    # 4. Bitcoin Script 생성 (간단한 예시)
    # 실제로는 BitVMX 프로토콜에 따른 복잡한 스크립트가 필요
    script_data = {
        "input": input_hex,
        "output": payout,
        "steps": steps,
        "program_hash": hashlib.sha256(open(elf_path, 'rb').read()).hexdigest()
    }
    
    # 5. 증명 데이터 구성
    proof = {
        "type": "bitvmx_option_settlement",
        "version": 1,
        "option": {
            "type": "CALL" if option_type == 0 else "PUT",
            "strike": strike_price_usd,
            "spot": spot_price_usd,
            "quantity": quantity
        },
        "execution": execution_result,
        "script": script_data,
        "timestamp": int(subprocess.check_output(['date', '+%s']).strip())
    }
    
    return proof

# 테스트 케이스들
test_cases = [
    # Call ITM
    {"type": 0, "strike": 50000, "spot": 52000, "quantity": 1.0},
    # Put ITM
    {"type": 1, "strike": 50000, "spot": 48000, "quantity": 2.0},
    # Call OTM
    {"type": 0, "strike": 52000, "spot": 50000, "quantity": 0.5},
]

print("=== BitVMX 옵션 정산 증명 생성 ===\n")

for i, test in enumerate(test_cases):
    print(f"\n테스트 {i+1}:")
    proof = generate_option_settlement_proof(
        test["type"],
        test["strike"],
        test["spot"],
        test["quantity"]
    )
    
    if proof:
        # 증명을 파일로 저장
        filename = f"proof_{i+1}.json"
        with open(filename, 'w') as f:
            json.dump(proof, f, indent=2)
        print(f"증명 저장됨: {filename}")
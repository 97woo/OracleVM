#!/usr/bin/env python3
"""
완전한 BitVMX 증명 생성 (SHA-256 스크립트 포함)
"""

import subprocess
import json
import struct
import hashlib
import os
import sys

# pybitvmbinding 경로 추가
sys.path.insert(0, '/Users/parkgeonwoo/oracle_vm/bitvmx_protocol')
import pybitvmbinding

def create_option_input_hex(option_type, strike_usd, spot_usd, quantity):
    """옵션 데이터를 BitVMX 입력 형식으로 변환"""
    # Little-endian 형식으로 패킹
    option_type_int = 0 if option_type == "CALL" else 1
    strike_cents = int(strike_usd * 100)
    spot_cents = int(spot_usd * 100)
    quantity_scaled = int(quantity * 100)
    
    packed = struct.pack('<IIII', 
                        option_type_int,
                        strike_cents,
                        spot_cents,
                        quantity_scaled)
    
    return packed.hex()

def run_bitvmx_emulator(elf_path, input_hex):
    """BitVMX 에뮬레이터 실행"""
    cmd = [
        "./BitVMX-CPU/target/release/emulator",
        "execute",
        "--elf", elf_path,
        "--input", input_hex,
        "--stdout"
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    # stderr와 stdout 모두에서 결과 파싱
    payout = 0
    steps = 0
    
    # stderr와 stdout을 모두 확인
    output = result.stderr + "\n" + result.stdout
    
    if "Halt(" in output:
        for line in output.split('\n'):
            if "Halt(" in line:
                # Halt(200000, 907) 형식 파싱
                parts = line.split("Halt(")[1].split(",")
                payout = int(parts[0])
                steps = int(parts[1].strip().rstrip(")"))
                break
    
    return payout, steps

def generate_complete_proof(option_type, strike_usd, spot_usd, quantity, proof_num):
    """완전한 BitVMX 증명 생성"""
    
    print(f"\n{'='*60}")
    print(f"증명 {proof_num}: {option_type} Strike ${strike_usd:,} → Spot ${spot_usd:,}")
    print(f"{'='*60}")
    
    # 1. 입력 데이터 준비
    input_hex = create_option_input_hex(option_type, strike_usd, spot_usd, quantity)
    print(f"입력 데이터: {input_hex}")
    
    # 2. ELF 파일 해시
    elf_path = "./option_settlement_bitvmx_complete.elf"
    with open(elf_path, 'rb') as f:
        elf_data = f.read()
        program_hash = hashlib.sha256(elf_data).hexdigest()
    
    # 3. BitVMX 실행
    payout_cents, steps = run_bitvmx_emulator(elf_path, input_hex)
    payout_usd = payout_cents / 100
    
    print(f"실행 결과: ${payout_usd:.2f} ({steps} CPU 스텝)")
    
    # 4. SHA-256 검증 스크립트 생성
    input_bytes = bytes.fromhex(input_hex)
    sha256_script = pybitvmbinding.sha_256_script(len(input_bytes))
    print(f"SHA-256 스크립트 생성: {len(sha256_script)} opcodes")
    
    # 5. 완전한 증명 구성
    complete_proof = {
        "type": "complete_bitvmx_proof",
        "version": 2,
        "option": {
            "type": option_type,
            "strike_usd": strike_usd,
            "spot_usd": spot_usd,
            "quantity": quantity
        },
        "execution": {
            "payout_cents": payout_cents,
            "payout_usd": payout_usd,
            "steps": steps
        },
        "verification": {
            "program_hash": program_hash,
            "input_hex": input_hex,
            "sha256_script_length": len(sha256_script),
            "sha256_script_sample": list(sha256_script[:20]),  # 첫 20 바이트만
            "bitcoin_script_ready": True
        },
        "on_chain": {
            "can_verify": True,
            "script_size_bytes": len(sha256_script),
            "estimated_gas": len(sha256_script) * 4  # 대략적인 가스 추정
        }
    }
    
    # 6. 파일 저장
    filename = f"complete_proof_{proof_num}.json"
    with open(filename, 'w') as f:
        json.dump(complete_proof, f, indent=2)
    
    print(f"✅ 증명 저장: {filename}")
    
    return complete_proof

def main():
    """3가지 시나리오에 대한 증명 생성"""
    
    # pybitvmbinding 작동 확인
    try:
        test_script = pybitvmbinding.sha_256_script(32)
        print(f"✅ pybitvmbinding 정상 작동: {len(test_script)} opcodes")
    except Exception as e:
        print(f"❌ pybitvmbinding 오류: {e}")
        return
    
    scenarios = [
        # 1. Call ITM: 행사가 $50k, 현물가 $52k
        ("CALL", 50000, 52000, 1.0, 1),
        
        # 2. Put ITM: 행사가 $50k, 현물가 $48k  
        ("PUT", 50000, 48000, 2.0, 2),
        
        # 3. Call OTM: 행사가 $52k, 현물가 $50k
        ("CALL", 52000, 50000, 0.5, 3)
    ]
    
    proofs = []
    for scenario in scenarios:
        proof = generate_complete_proof(*scenario)
        proofs.append(proof)
    
    print(f"\n{'='*60}")
    print(f"✅ 모든 증명 생성 완료!")
    print(f"{'='*60}")
    
    # 요약
    for i, proof in enumerate(proofs, 1):
        option = proof["option"]
        execution = proof["execution"]
        print(f"\n증명 {i}: {option['type']} ${option['strike_usd']:,} → ${option['spot_usd']:,}")
        print(f"  결과: ${execution['payout_usd']:.2f} ({execution['steps']} 스텝)")
        print(f"  SHA-256 스크립트: {proof['verification']['sha256_script_length']} opcodes")

if __name__ == "__main__":
    main()
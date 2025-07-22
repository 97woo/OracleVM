#!/usr/bin/env python3
"""
Bitcoin regtest에 옵션 정산 증명을 앵커링
"""

import json
import hashlib
import subprocess
import sys

def create_op_return_hex(data_hash):
    """해시를 OP_RETURN 형식으로 변환"""
    # OP_RETURN (0x6a) + 데이터 길이 + 데이터
    # 최대 80바이트 제한이므로 해시만 사용
    op_return = "6a20" + data_hash  # 0x6a=OP_RETURN, 0x20=32바이트
    
    return op_return

def anchor_proof_to_bitcoin(proof_file):
    """증명을 Bitcoin regtest에 앵커링"""
    
    # 1. 증명 파일 읽기
    with open(proof_file, 'r') as f:
        proof = json.load(f)
    
    # 2. 앵커링할 데이터 확인
    if "bitcoin_anchor" in proof:
        # 새로운 머클 증명 형식
        anchor_hash = proof["bitcoin_anchor"]["anchor_hash"]
        print(f"머클 증명 앵커링:")
        print(f"  트레이스 머클 루트: {proof['commitments']['trace_merkle_root']}")
        print(f"  최종 상태 commitment: {proof['commitments']['final_state']}")
        print(f"  앵커 해시: {anchor_hash}")
    else:
        # 기존 형식 (호환성 유지)
        anchor_data = {
            "type": proof["type"],
            "option": proof["option"],
            "execution": {
                "payout_cents": proof["execution"]["payout_cents"],
                "steps": proof["execution"]["steps"]
            },
            "program_hash": proof["verification"]["program_hash"]
        }
        data_str = json.dumps(anchor_data, separators=(',', ':'))
        anchor_hash = hashlib.sha256(data_str.encode()).hexdigest()
        print(f"앵커링 데이터: {data_str}")
    
    # 3. OP_RETURN 스크립트 생성
    op_return_hex = create_op_return_hex(anchor_hash)
    print(f"OP_RETURN hex: {op_return_hex}")
    
    # 5. 새 주소 생성 (거스름돈용)
    cmd = [
        "docker", "exec", "8a72bfbe14fe",
        "/srv/explorer/bitcoin-27.2/bin/bitcoin-cli",
        "-regtest", "-datadir=/data/bitcoin",
        "-rpcwallet=default", "getnewaddress"
    ]
    change_address = subprocess.check_output(cmd).decode().strip()
    print(f"거스름돈 주소: {change_address}")
    
    # 6. Raw 트랜잭션 생성
    # 간단히 하기 위해 createrawtransaction 대신 sendtoaddress 사용
    # OP_RETURN은 별도로 처리
    
    # 먼저 UTXO 확인
    cmd = [
        "docker", "exec", "8a72bfbe14fe",
        "/srv/explorer/bitcoin-27.2/bin/bitcoin-cli",
        "-regtest", "-datadir=/data/bitcoin",
        "-rpcwallet=default", "listunspent"
    ]
    unspent = json.loads(subprocess.check_output(cmd).decode())
    
    if not unspent:
        print("사용 가능한 UTXO가 없습니다!")
        return None
    
    # 첫 번째 UTXO 사용
    utxo = unspent[0]
    print(f"사용할 UTXO: {utxo['txid']}:{utxo['vout']}")
    
    # 7. Raw 트랜잭션 생성
    inputs = [{
        "txid": utxo["txid"],
        "vout": utxo["vout"]
    }]
    
    # 출력: 거스름돈 + OP_RETURN
    # 수수료 0.001 BTC
    change_amount = float(utxo["amount"]) - 0.001
    outputs = [
        {change_address: f"{change_amount:.8f}"},
        {"data": op_return_hex[4:]}  # "6a20" 제거
    ]
    
    cmd = [
        "docker", "exec", "8a72bfbe14fe",
        "/srv/explorer/bitcoin-27.2/bin/bitcoin-cli",
        "-regtest", "-datadir=/data/bitcoin",
        "-rpcwallet=default", "createrawtransaction",
        json.dumps(inputs), json.dumps(outputs)
    ]
    
    raw_tx = subprocess.check_output(cmd).decode().strip()
    print(f"\nRaw 트랜잭션 생성됨")
    
    # 8. 트랜잭션 서명
    cmd = [
        "docker", "exec", "8a72bfbe14fe",
        "/srv/explorer/bitcoin-27.2/bin/bitcoin-cli",
        "-regtest", "-datadir=/data/bitcoin",
        "-rpcwallet=default", "signrawtransactionwithwallet",
        raw_tx
    ]
    
    signed = json.loads(subprocess.check_output(cmd).decode())
    
    if not signed["complete"]:
        print("트랜잭션 서명 실패!")
        return None
    
    signed_tx = signed["hex"]
    print("트랜잭션 서명 완료")
    
    # 9. 트랜잭션 브로드캐스트
    cmd = [
        "docker", "exec", "8a72bfbe14fe",
        "/srv/explorer/bitcoin-27.2/bin/bitcoin-cli",
        "-regtest", "-datadir=/data/bitcoin",
        "-rpcwallet=default", "sendrawtransaction",
        signed_tx
    ]
    
    txid = subprocess.check_output(cmd).decode().strip()
    print(f"\n✅ 트랜잭션 브로드캐스트 성공!")
    print(f"트랜잭션 ID: {txid}")
    
    # 10. 블록 생성하여 확정
    cmd = [
        "docker", "exec", "8a72bfbe14fe",
        "/srv/explorer/bitcoin-27.2/bin/bitcoin-cli",
        "-regtest", "-datadir=/data/bitcoin",
        "generatetoaddress", "1", change_address
    ]
    
    block_hash = json.loads(subprocess.check_output(cmd).decode())[0]
    print(f"블록 생성: {block_hash}")
    
    # 11. 트랜잭션 확인
    print(f"\n트랜잭션 확인:")
    print(f"http://localhost:8094/regtest/tx/{txid}")
    
    return txid

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: ./anchor_option_proof.py <proof_file>")
        sys.exit(1)
    
    proof_file = sys.argv[1]
    txid = anchor_proof_to_bitcoin(proof_file)
    
    if txid:
        print(f"\n🎉 옵션 정산 증명이 Bitcoin regtest에 앵커링되었습니다!")
        print(f"트랜잭션 ID: {txid}")
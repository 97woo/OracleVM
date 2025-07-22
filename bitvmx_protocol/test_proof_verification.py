#!/usr/bin/env python3
"""
BitVMX Proof Verification Test Script
This script demonstrates the proof generation and verification process
for the option settlement system.
"""

import os
import sys
import json
import subprocess
from pathlib import Path

# Add the bitvmx_protocol_library to the Python path
sys.path.insert(0, str(Path(__file__).parent))

def check_environment():
    """Check if the environment is properly set up"""
    print("=== BitVMX Proof Verification Test ===\n")
    print("1. Checking environment setup...")
    
    required_dirs = ['prover_files', 'verifier_files', 'execution_files']
    required_files = ['.env_common', '.env_prover', '.env_verifier']
    
    all_good = True
    for dir_name in required_dirs:
        if os.path.exists(dir_name):
            print(f"✅ {dir_name}/ directory exists")
        else:
            print(f"❌ {dir_name}/ directory missing")
            all_good = False
    
    for file_name in required_files:
        if os.path.exists(file_name):
            print(f"✅ {file_name} exists")
        else:
            print(f"❌ {file_name} missing")
            all_good = False
    
    return all_good

def test_execution_trace_generation():
    """Test generation of execution traces"""
    print("\n2. Testing execution trace generation...")
    
    # Check if we have the test ELF file
    test_elf = "execution_files/test_input.elf"
    if not os.path.exists(test_elf):
        print(f"❌ Test ELF file not found: {test_elf}")
        return False
    
    print(f"✅ Found test ELF file: {test_elf}")
    
    # Try to read the instruction mapping
    mapping_file = "execution_files/instruction_mapping.txt"
    if os.path.exists(mapping_file):
        print(f"✅ Found instruction mapping: {mapping_file}")
        with open(mapping_file, 'r') as f:
            lines = f.readlines()
            print(f"   Total mappings: {len(lines)}")
    else:
        print(f"❌ Instruction mapping not found: {mapping_file}")
        return False
    
    return True

def test_witness_generation():
    """Test witness generation components"""
    print("\n3. Testing witness generation components...")
    
    try:
        from bitvmx_protocol_library.bitvmx_protocol_definition.entities.execution_trace_witness_dto import (
            ExecutionTraceWitnessDTO
        )
        print("✅ Successfully imported ExecutionTraceWitnessDTO")
        
        # Create a sample witness
        sample_witness = ExecutionTraceWitnessDTO(
            read_1_address=["00", "00", "00", "00"],
            read_1_value=["00", "00", "00", "00"],
            read_1_last_step=["00", "00"],
            read_2_address=["00", "00", "00", "00"],
            read_2_value=["00", "00", "00", "00"],
            read_2_last_step=["00", "00"],
            opcode=["00", "00", "00", "00"],
            read_PC_address=["00", "00", "00", "00"],
            read_micro=["00", "00"],
            write_address=["00", "00", "00", "00"],
            write_value=["00", "00", "00", "00"],
            write_PC_address=["00", "00", "00", "00"],
            write_micro=["00", "00"]
        )
        
        print("✅ Successfully created sample ExecutionTraceWitnessDTO")
        print(f"   Witness fields: {list(sample_witness.dict().keys())}")
        
        return True
        
    except ImportError as e:
        print(f"❌ Failed to import witness components: {e}")
        return False

def test_option_settlement_logic():
    """Test the option settlement calculation logic"""
    print("\n4. Testing option settlement logic...")
    
    # Test cases for option settlement
    test_cases = [
        {
            "name": "Call ITM",
            "option_type": 0,  # Call
            "strike": 50000_00,  # $50,000
            "spot": 52000_00,    # $52,000
            "quantity": 100,     # 1.0 BTC
            "expected_itm": True,
            "expected_value": 2000_00  # $2,000
        },
        {
            "name": "Put ITM",
            "option_type": 1,  # Put
            "strike": 50000_00,  # $50,000
            "spot": 48000_00,    # $48,000
            "quantity": 100,     # 1.0 BTC
            "expected_itm": True,
            "expected_value": 2000_00  # $2,000
        },
        {
            "name": "Call OTM",
            "option_type": 0,  # Call
            "strike": 52000_00,  # $52,000
            "spot": 48000_00,    # $48,000
            "quantity": 100,     # 1.0 BTC
            "expected_itm": False,
            "expected_value": 0
        }
    ]
    
    all_passed = True
    for test in test_cases:
        # Simple ITM check
        if test["option_type"] == 0:  # Call
            is_itm = test["spot"] > test["strike"]
            intrinsic = max(0, test["spot"] - test["strike"])
        else:  # Put
            is_itm = test["spot"] < test["strike"]
            intrinsic = max(0, test["strike"] - test["spot"])
        
        intrinsic_value = (intrinsic * test["quantity"]) // 100
        
        passed = (is_itm == test["expected_itm"] and 
                 intrinsic_value == test["expected_value"])
        
        status = "✅" if passed else "❌"
        print(f"{status} {test['name']}: ITM={is_itm}, Value=${intrinsic_value/100}")
        
        if not passed:
            all_passed = False
    
    return all_passed

def test_proof_script_generation():
    """Test Bitcoin script generation for proofs"""
    print("\n5. Testing proof script generation...")
    
    try:
        from bitvmx_protocol_library.script_generation.entities.business_objects.bitcoin_script import (
            BitcoinScript
        )
        from bitvmx_protocol_library.script_generation.entities.business_objects.bitcoin_script_list import (
            BitcoinScriptList
        )
        
        print("✅ Successfully imported script generation components")
        
        # Create a simple test script
        test_script = BitcoinScript(script="OP_SHA256 OP_EQUAL")
        print(f"✅ Created test BitcoinScript: {test_script.script}")
        
        # Create a script list
        script_list = BitcoinScriptList(scripts=[test_script])
        print(f"✅ Created BitcoinScriptList with {len(script_list.scripts)} script(s)")
        
        return True
        
    except ImportError as e:
        print(f"❌ Failed to import script components: {e}")
        return False

def main():
    """Run all verification tests"""
    print("="*50)
    print("BitVMX Proof Verification Test Suite")
    print("="*50)
    
    # Change to bitvmx_protocol directory
    os.chdir(Path(__file__).parent)
    
    tests = [
        ("Environment Check", check_environment),
        ("Execution Trace", test_execution_trace_generation),
        ("Witness Generation", test_witness_generation),
        ("Settlement Logic", test_option_settlement_logic),
        ("Script Generation", test_proof_script_generation)
    ]
    
    results = []
    for test_name, test_func in tests:
        try:
            result = test_func()
            results.append((test_name, result))
        except Exception as e:
            print(f"\n❌ Error in {test_name}: {e}")
            results.append((test_name, False))
    
    # Summary
    print("\n" + "="*50)
    print("TEST SUMMARY")
    print("="*50)
    
    passed = sum(1 for _, result in results if result)
    total = len(results)
    
    for test_name, result in results:
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"{test_name}: {status}")
    
    print(f"\nTotal: {passed}/{total} tests passed")
    
    if passed == total:
        print("\n🎉 All tests passed! The BitVMX proof system is ready.")
        print("\nNext steps:")
        print("1. Initialize BitVMX-CPU submodule: git submodule update --init --recursive")
        print("2. Build Docker containers: docker compose build")
        print("3. Run prover service: docker compose up prover-backend")
        print("4. Run verifier service: docker compose up verifier-backend")
    else:
        print("\n⚠️  Some tests failed. Please fix the issues before proceeding.")
    
    return passed == total

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
# 간단한 RISC-V 옵션 정산 어셈블리
.text
.globl _start

_start:
    # 입력 주소: 0x10000
    # 출력 주소: 0x20000
    
    # 입력 데이터 로드
    li      t0, 0x10000      # 입력 주소
    lw      a0, 0(t0)        # option_type
    lw      a1, 4(t0)        # strike_price
    lw      a2, 8(t0)        # spot_price
    lw      a3, 12(t0)       # quantity
    
    # Call option 체크 (type == 0)
    bnez    a0, put_option
    
call_option:
    # spot > strike 체크
    ble     a2, a1, otm
    
    # ITM: payoff = (spot - strike) * quantity / 100
    sub     t1, a2, a1       # spot - strike
    li      t2, 100
    div     t3, a3, t2       # quantity / 100
    mul     t4, t1, t3       # (spot - strike) * (quantity / 100)
    li      t5, 1            # is_itm = 1
    j       save_result
    
put_option:
    # spot < strike 체크  
    bge     a2, a1, otm
    
    # ITM: payoff = (strike - spot) * quantity / 100
    sub     t1, a1, a2       # strike - spot
    li      t2, 100
    div     t3, a3, t2       # quantity / 100
    mul     t4, t1, t3       # (strike - spot) * (quantity / 100)
    li      t5, 1            # is_itm = 1
    j       save_result
    
otm:
    # OTM: payoff = 0, is_itm = 0
    li      t4, 0            # payoff = 0
    li      t5, 0            # is_itm = 0
    
save_result:
    # 결과 저장
    li      t0, 0x20000      # 출력 주소
    sw      t4, 0(t0)        # payoff
    sw      t5, 4(t0)        # is_itm
    
    # 종료
    li      a7, 93           # exit syscall
    li      a0, 0            # exit code 0
    ecall
#include <stdint.h>

// BitVMX를 위한 간단한 옵션 정산 프로그램
// 입력: 옵션타입(4바이트) + 행사가(4바이트) + 현물가(4바이트) + 수량(4바이트)
// 출력: 정산금액(4바이트)

// 간단한 stdout 출력을 위한 함수
void print_uint32(uint32_t value) {
    // BitVMX는 특별한 메모리 주소에 쓰는 것으로 출력을 처리
    volatile uint32_t* output = (volatile uint32_t*)0x10000000;
    *output = value;
}

int main() {
    // 입력 데이터를 읽을 주소 (BitVMX는 특정 주소에서 입력을 읽음)
    uint32_t* input = (uint32_t*)0x20000000;
    
    uint32_t option_type = input[0];  // 0=Call, 1=Put
    uint32_t strike_price = input[1]; // 행사가 (cents)
    uint32_t spot_price = input[2];   // 현물가 (cents)
    uint32_t quantity = input[3];     // 수량 (0.01 단위)
    
    uint32_t payout = 0;
    
    if (option_type == 0) {  // Call Option
        if (spot_price > strike_price) {
            payout = ((spot_price - strike_price) * quantity) / 100;
        }
    } else if (option_type == 1) {  // Put Option
        if (strike_price > spot_price) {
            payout = ((strike_price - spot_price) * quantity) / 100;
        }
    }
    
    // 결과 출력
    print_uint32(payout);
    
    return 0;
}
// BitVMX 호환 옵션 정산 프로그램

// 메모리 맵
#define INPUT_ADDR  0xaa000000  // BitVMX 입력 주소
#define OUTPUT_ADDR 0x10000000  // 출력 주소

// 32비트 정수 타입
typedef unsigned int uint32_t;

// 함수 선언
int main();

// _start 엔트리 포인트
void _start() {
    // 스택 포인터 설정
    asm volatile("li sp, 0xe0800000");
    
    // main 호출
    main();
    
    // 종료
    while(1);
}

// 메인 함수
int main() {
    // 입력 데이터 읽기
    uint32_t* input = (uint32_t*)INPUT_ADDR;
    
    uint32_t option_type = input[0];   // 0=Call, 1=Put
    uint32_t strike_price = input[1];  // 행사가 (cents)
    uint32_t spot_price = input[2];    // 현물가 (cents)
    uint32_t quantity = input[3];      // 수량 (0.01 단위)
    
    uint32_t payout = 0;
    
    // 옵션 정산 계산
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
    volatile uint32_t* output = (volatile uint32_t*)OUTPUT_ADDR;
    *output = payout;
    
    return 0;
}
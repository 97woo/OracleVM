#include <stdint.h>

// 입력 데이터 구조
typedef struct {
    uint32_t option_type;    // 0 = Call, 1 = Put
    uint32_t strike_price;   // 행사가 (USD * 100)
    uint32_t spot_price;     // 현재가 (USD * 100)  
    uint32_t quantity;       // 수량 (unit * 100)
} OptionInput;

// 출력 데이터 구조
typedef struct {
    uint32_t payoff;         // 지급액 (USD * 100)
    uint32_t is_itm;         // ITM 여부 (1 = ITM, 0 = OTM)
} OptionOutput;

// 메인 정산 함수
void settle_option(const OptionInput* input, OptionOutput* output) {
    if (input->option_type == 0) {  // Call Option
        if (input->spot_price > input->strike_price) {
            // ITM: 지급액 = (현재가 - 행사가) * 수량
            output->payoff = ((input->spot_price - input->strike_price) * input->quantity) / 100;
            output->is_itm = 1;
        } else {
            // OTM: 지급액 = 0
            output->payoff = 0;
            output->is_itm = 0;
        }
    } else {  // Put Option
        if (input->spot_price < input->strike_price) {
            // ITM: 지급액 = (행사가 - 현재가) * 수량
            output->payoff = ((input->strike_price - input->spot_price) * input->quantity) / 100;
            output->is_itm = 1;
        } else {
            // OTM: 지급액 = 0
            output->payoff = 0;
            output->is_itm = 0;
        }
    }
}

// RISC-V 에뮬레이터 진입점
int main() {
    // 입력 데이터 읽기 (메모리 매핑된 주소에서)
    OptionInput* input = (OptionInput*)0x10000;
    OptionOutput output;
    
    // 옵션 정산 실행
    settle_option(input, &output);
    
    // 결과 출력 (메모리 매핑된 주소로)
    OptionOutput* result = (OptionOutput*)0x20000;
    result->payoff = output.payoff;
    result->is_itm = output.is_itm;
    
    return 0;
}
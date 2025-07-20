# Contract Module Test Suite

## Unit Tests Structure

### 1. Option Tests (`unit/option_test.rs`)
- **Option Creation**: Test creating Call and Put options
- **Option Validation**: Validate strike price and quantity boundaries
- **Option Settlement**: Test ITM/OTM calculations for both Call and Put options

### 2. Pool Tests (`unit/pool_test.rs`)
- **Pool State Management**: Test pool initialization and state tracking
- **Pool Operations**: Test liquidity operations (add, lock, release)
- **Pool Calculations**: Test collateral requirements and profit/loss calculations

### 3. Manager Tests (`unit/manager_test.rs`)
- **Contract Manager**: Test the integrated manager functionality
- **Option Lifecycle**: Test full option lifecycle from creation to settlement
- **System Status**: Test utilization rates and system reporting

## Running Tests

### Run all library tests:
```bash
cargo test --lib -- --nocapture
```

### Run specific test module:
```bash
cargo test simple_contract::tests -- --nocapture
```

### Run standalone tests:
```bash
cargo test --test standalone_test -- --nocapture
```

## Test Coverage

| Component | Tests | Status |
|-----------|-------|---------|
| SimpleOption | 12 | ✅ |
| SimplePoolState | 10 | ✅ |
| SimpleContractManager | 15 | ✅ |
| Bitcoin Scripts | 10 | ⚠️ (compilation issues) |

## Key Test Scenarios

1. **Option Creation**
   - Valid option creation with sufficient liquidity
   - Rejection when insufficient liquidity
   - Proper collateral locking

2. **Settlement Logic**
   - ITM Call: Spot > Strike → Payout
   - OTM Call: Spot < Strike → No payout
   - ITM Put: Spot < Strike → Payout
   - OTM Put: Spot > Strike → No payout

3. **Pool Management**
   - Liquidity addition and tracking
   - Collateral locking and release
   - Premium collection
   - Payout processing

4. **System Health**
   - Utilization rate calculations
   - Profit/loss tracking
   - Active options monitoring
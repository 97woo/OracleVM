# BTCFi Option Settlement System Architecture

## Overview

This document describes the complete architecture of the BTCFi option settlement system that enables trustless, automated option settlement on Bitcoin Layer 1 using BitVMX.

## System Components

### 1. Oracle Layer (Offchain)
- **Location**: `crates/oracle-node/`
- **Purpose**: Collect and aggregate price data from multiple exchanges
- **Key Features**:
  - Multi-exchange price collection (Binance, Coinbase, Kraken)
  - 2/3 consensus validation
  - Real-time price feed aggregation
  - WebSocket support for streaming data

### 2. Contract Layer (Onchain)
- **Location**: `contracts/`
- **Purpose**: Manage option contracts and settlements
- **Key Features**:
  - Buyer-only option system (no order matching)
  - Delta-neutral pool management
  - Automatic ITM/OTM determination
  - Target theta-based premium calculation

### 3. BitVMX Layer (Offchain → Onchain)
- **Location**: `bitvmx_protocol/`
- **Purpose**: Generate and verify proofs for option settlement
- **Key Features**:
  - RISC-V based computation verification
  - Option settlement logic execution
  - Proof generation for Bitcoin L1 anchoring
  - Mock implementation for testing

### 4. Calculation Layer (Offchain)
- **Location**: `calculation/`
- **Purpose**: Option pricing and risk calculations
- **Key Features**:
  - Black-Scholes pricing model
  - Greeks calculation (Delta, Gamma, Theta, Vega)
  - Target theta to IV solver (Newton-Raphson)
  - REST API for pricing data

## Data Flow

### 1. Option Creation Flow
```
User → Contract Layer → Calculation Layer → BitVMX Layer
         ↓                    ↓                   ↓
    Create Option      Calculate Premium    Pre-sign Settlement
```

### 2. Price Update Flow
```
Exchanges → Oracle Node → Aggregator → Contract/Calculation
    ↓           ↓            ↓              ↓
 Raw Price   Validate    Consensus    Update Greeks
```

### 3. Settlement Flow (At Expiry)
```
Oracle → BitVMX → Contract → User
   ↓        ↓         ↓        ↓
Price   Generate   Execute  Receive
Data     Proof   Settlement  Payout
```

## Key Design Decisions

### 1. Buyer-Only Options
- No order book or matching engine needed
- Pool automatically sells options at calculated premiums
- Simplifies settlement and reduces complexity

### 2. Target Theta Pricing
- Options priced to achieve specific daily decay rate
- Ensures predictable returns for the pool
- Uses Newton-Raphson method to find appropriate IV

### 3. 2/3 Consensus Oracle
- Requires agreement from at least 2 of 3 exchanges
- Protects against single exchange manipulation
- 5% deviation limit for price anomaly detection

### 4. BitVMX Integration
- Enables trustless computation verification on Bitcoin
- RISC-V programs compiled to Bitcoin script
- Pre-signed transactions enable automatic settlement

## Security Considerations

### 1. Price Manipulation Protection
- Multi-exchange consensus requirement
- Timestamp synchronization checks (1 minute tolerance)
- Price deviation limits (5% from median)

### 2. Settlement Security
- BitVMX proofs ensure correct calculation
- Pre-signed transactions prevent fund lock
- Automatic execution at expiry

### 3. Pool Risk Management
- Delta-neutral hedging strategy
- Gamma risk monitoring
- Position limits per option type

## Testing Strategy

### 1. Unit Tests
- Individual component testing
- Price aggregation logic
- Option pricing calculations
- Settlement logic

### 2. Integration Tests
- End-to-end option lifecycle
- Multi-exchange price consensus
- Settlement execution
- Payout calculations

### 3. Mock BitVMX Testing
- Simulated proof generation
- Settlement verification
- Performance benchmarking

## Future Enhancements

### 1. Production BitVMX
- Replace mock with actual RISC-V execution
- Bitcoin mainnet anchoring
- Challenge-response protocol

### 2. Advanced Features
- Multi-asset options
- Exotic option types
- Cross-chain settlement
- Liquidity mining incentives

### 3. Risk Management
- Automated rebalancing
- Dynamic position limits
- Advanced hedging strategies
- Real-time risk dashboard

## Performance Metrics

### Current System (Mock)
- Option creation: < 100ms
- Price consensus: < 50ms per update
- Settlement calculation: < 10ms
- Proof generation (mock): < 5ms

### Target Production Metrics
- BitVMX proof generation: < 1 second
- Bitcoin confirmation: ~10 minutes
- End-to-end settlement: < 15 minutes

## Deployment Architecture

### Development
```
├── Oracle Nodes (3x)
├── Aggregator Service
├── Contract Layer
├── BitVMX Prover/Verifier
└── Calculation API
```

### Production (Planned)
```
├── Distributed Oracle Network
├── Redundant Aggregators
├── Bitcoin L1 Contracts
├── BitVMX Proof Network
└── Global API Gateway
```

## Conclusion

The BTCFi option settlement system represents a novel approach to bringing DeFi primitives directly to Bitcoin Layer 1. By combining multi-source oracles, automated market making, and BitVMX verification, we enable trustless, efficient option trading without requiring additional layers or sidechains.

The modular architecture ensures each component can be independently scaled and improved while maintaining system integrity. The successful implementation of mock testing demonstrates the viability of the approach, with the path to production deployment clearly defined.
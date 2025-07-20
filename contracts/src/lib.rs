pub mod simple_contract;
pub mod bitcoin_option;
pub mod bitvmx_bridge;
pub mod testnet_deployer;

pub use simple_contract::{
    OptionStatus, SimpleContractManager, SimpleOption, SimplePoolState,
};
pub use oracle_vm_common::types::OptionType;

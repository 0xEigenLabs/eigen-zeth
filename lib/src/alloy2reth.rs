use ethers_core::types::H160;
use revm::primitives::{Log as RevmLog, B160 as RevmB160, B256 as RevmB256, U256 as RevmU256};
use zeth_primitives::receipt::Log;
use zeth_primitives::{
    access_list::{AccessList, AccessListItem},
    Address, FixedBytes, B256,
};

/*
`From<revm::revm_primitives::Log>` is not implemented for `zeth_primitives::receipt::Log`, which is required by `revm::revm_primitives::Log: Into<_>`
    |
*/
pub fn to_address(value: RevmB160) -> Address {
    let value: H160 = value.into();
    Address(FixedBytes(value.to_fixed_bytes()))
}

pub fn from_address(value: Address) -> RevmB160 {
    RevmB160::from_slice(&value.0 .0)
}

pub fn to_b256(value: RevmB256) -> B256 {
    B256::from_slice(&value.to_fixed_bytes())
}

pub fn from_b256(value: B256) -> RevmB256 {
    RevmB256::from_slice(&value.0)
}

pub fn to_log(log: RevmLog) -> Log {
    Log {
        address: to_address(log.address),
        topics: log.topics.iter().map(|e| to_b256(*e)).collect(),
        data: log.data.into(),
    }
}

pub fn from_access_list(access_list: Vec<(RevmB160, Vec<RevmU256>)>) -> AccessList {
    AccessList(
        access_list
            .iter()
            .map(|(k, v)| AccessListItem {
                address: to_address(*k),
                storage_keys: v.iter().map(|v| (*v).into()).collect(),
            })
            .collect(),
    )
}

pub fn to_access_list(access_list: AccessList) -> Vec<(RevmB160, Vec<RevmU256>)> {
    access_list
        .0
        .iter()
        .map(|v| {
            (
                from_address(v.address),
                v.storage_keys.iter().map(|vv| (*vv).into()).collect(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_b160() {
        let value = RevmB160::random();
        assert_eq!(value.clone(), from_address(to_address(value)));
    }
    #[test]
    fn test_revm_b256() {
        let value = RevmB256::random();
        assert_eq!(value.clone(), from_b256(to_b256(value)));
    }
}

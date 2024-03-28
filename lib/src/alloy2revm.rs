use zeth_primitives::{Address};
use revm::primitives::U160;

pub fn to_u160(addr: Address) -> U160 {
    addr.into()
}


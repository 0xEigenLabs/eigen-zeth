use crate::settlement::BatchData;
use anyhow::Result;
use ethers_core::types::{Address, Bytes, U256};
use ethers_core::utils::hex;
use reqwest::Client;
use serde_json::json;

pub struct CustomClient {
    pub client: Client,
    pub url: String,
}

const SUCCESS_STATUS: u64 = 1;

impl CustomClient {
    pub fn new(url: String) -> Self {
        let client = Client::new();
        CustomClient { client, url }
    }

    pub async fn get_last_rollup_exit_root(&self) -> Result<[u8; 32]> {
        // TODO
        todo!()
    }

    pub async fn get_global_exit_root(&self) -> Result<[u8; 32]> {
        let respose = self
            .client
            .get(format!("{}/get-global-exit-root", self.url.clone()))
            .send()
            .await;
        let mut last_global_exit_root = [0u8; 32];

        match respose {
            Ok(res) => {
                if res.status().is_success() {
                    let body = res.text().await?;
                    let parsed_json: serde_json::Value = serde_json::from_str(&body).unwrap();
                    let global_exit_root =
                        parsed_json["global_exit_root"].as_str().unwrap_or_default();
                    log::debug!("global_exit_root: {:?}", global_exit_root);
                    let global_exit_root_bytes =
                        hex::decode(&global_exit_root[2..]).expect("Failed to decode hex string");

                    last_global_exit_root.copy_from_slice(&global_exit_root_bytes);
                } else {
                    log::error!("Request failed, response: {:?}", res);
                }
            }
            Err(e) => {
                log::error!("Request error: {:?}", e);
            }
        }
        Ok(last_global_exit_root)
    }

    pub async fn bridge_asset(
        &self,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        token: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        let body = json!({
            "destination_network": destination_network,
            "destination_address": destination_address,
            "amount": amount.to_string(),
            "token": token,
            "force_update_global_exit_root": force_update_global_exit_root,
            "calldata": calldata.clone(),
        });

        let response = self
            .client
            .post(format!("{}/bridge-asset", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(parsed_resp) => {
                            if let Some(status) = parsed_resp.get("status").and_then(|s| s.as_u64())
                            {
                                if status == SUCCESS_STATUS {
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Contract call failed with status: {}",
                                        status
                                    ))
                                }
                            } else {
                                Err(anyhow::anyhow!("Response missing 'status' field"))
                            }
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to parse response: {:?}", e)),
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn bridge_message(
        &self,
        destination_network: u32,
        destination_address: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        let body = json!({
            "destination_network": destination_network,
            "destination_address": destination_address,
            "force_update_global_exit_root": force_update_global_exit_root,
            "calldata": calldata,
        });

        let response = self
            .client
            .post(format!("{}/bridge-message", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(parsed_resp) => {
                            if let Some(status) = parsed_resp.get("status").and_then(|s| s.as_u64())
                            {
                                if status == SUCCESS_STATUS {
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Contract call failed with status: {}",
                                        status
                                    ))
                                }
                            } else {
                                Err(anyhow::anyhow!("Response missing 'status' field"))
                            }
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to parse response: {:?}", e)),
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn claim_asset(
        &self,
        smt_proof: [[u8; 32]; 32],
        index: u32,
        mainnet_exit_root: [u8; 32],
        rollup_exit_root: [u8; 32],
        origin_network: u32,
        origin_token_address: Address,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        metadata: Bytes,
    ) -> Result<()> {
        let body = json!({
            "smt_proof": smt_proof,
            "index": index,
            "mainnet_exit_root": mainnet_exit_root,
            "rollup_exit_root": rollup_exit_root,
            "origin_network": origin_network,
            "origin_token_address": origin_token_address,
            "destination_network": destination_network,
            "destination_address": destination_address,
            "amount": amount,
            "metadata": metadata,
        });

        let response = self
            .client
            .post(format!("{}/claim-asset", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(parsed_resp) => {
                            if let Some(status) = parsed_resp.get("status").and_then(|s| s.as_u64())
                            {
                                if status == SUCCESS_STATUS {
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Contract call failed with status: {}",
                                        status
                                    ))
                                }
                            } else {
                                Err(anyhow::anyhow!("Response missing 'status' field"))
                            }
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to parse response: {:?}", e)),
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn claim_message(
        &self,
        smt_proof: [[u8; 32]; 32],
        index: u32,
        mainnet_exit_root: [u8; 32],
        rollup_exit_root: [u8; 32],
        origin_network: u32,
        origin_address: Address,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        metadata: Bytes,
    ) -> Result<()> {
        let body = json!({
            "smt_proof": smt_proof,
            "index": index,
            "mainnet_exit_root": mainnet_exit_root,
            "rollup_exit_root": rollup_exit_root,
            "origin_network": origin_network,
            "origin_address": origin_address,
            "destination_network": destination_network,
            "destination_address": destination_address,
            "amount": amount,
            "metadata": metadata,
        });

        let response = self
            .client
            .post(format!("{}/claim-message", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(parsed_resp) => {
                            if let Some(status) = parsed_resp.get("status").and_then(|s| s.as_u64())
                            {
                                if status == SUCCESS_STATUS {
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Contract call failed with status: {}",
                                        status
                                    ))
                                }
                            } else {
                                Err(anyhow::anyhow!("Response missing 'status' field"))
                            }
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to parse response: {:?}", e)),
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn update_exit_root(&self, network: u32, new_root: [u8; 32]) -> Result<()> {
        let body = json!({
            "network": network,
            "new_root": format!("0x{}", hex::encode(new_root))
        });

        let response = self
            .client
            .post(format!("{}/update-exit-root", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(parsed_resp) => {
                            if let Some(status) = parsed_resp.get("status").and_then(|s| s.as_u64())
                            {
                                if status == SUCCESS_STATUS {
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Contract call failed with status: {}",
                                        status
                                    ))
                                }
                            } else {
                                Err(anyhow::anyhow!("Response missing 'status' field"))
                            }
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to parse response: {:?}", e)),
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn sequence_batches(&self, batches: Vec<BatchData>) -> Result<()> {
        let batches: Vec<_> = batches
            .into_iter()
            .map(|batch| {
                json!({
                    "transactions": format!("0x{}", hex::encode(batch.transactions)),
                    "global_exit_root": format!("0x{}", hex::encode(batch.global_exit_root)),
                    "timestamp": batch.timestamp
                })
            })
            .collect();

        let body = json!({
            "batches": batches
        });

        log::debug!("sequence_batches body: {:?}", body);

        let response = self
            .client
            .post(format!("{}/sequence-batches", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(parsed_resp) => {
                            if let Some(status) = parsed_resp.get("status").and_then(|s| s.as_u64())
                            {
                                if status == SUCCESS_STATUS {
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Contract call failed with status: {}",
                                        status
                                    ))
                                }
                            } else {
                                Err(anyhow::anyhow!("Response missing 'status' field"))
                            }
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to parse response: {:?}", e)),
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn verify_batches(
        &self,
        pending_state_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        proof: String,
        input: String,
    ) -> Result<()> {
        let body = json!({
            "pending_state_num": pending_state_num,
            "init_num_batch": init_num_batch,
            "final_new_batch": final_new_batch,
            "new_local_exit_root": format!("0x{}", hex::encode(new_local_exit_root)),
            "new_state_root": format!("0x{}", hex::encode(new_state_root)),
            "proof": proof,
            "input": input
        });

        log::debug!("verify_batches body: {:?}", body);

        let response = self
            .client
            .post(format!("{}/verify-batches", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(parsed_resp) => {
                            if let Some(status) = parsed_resp.get("status").and_then(|s| s.as_u64())
                            {
                                if status == SUCCESS_STATUS {
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Contract call failed with status: {}",
                                        status
                                    ))
                                }
                            } else {
                                Err(anyhow::anyhow!("Response missing 'status' field"))
                            }
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to parse response: {:?}", e)),
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn verify_batches_trusted_aggregator(
        &self,
        pending_state_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        proof: String,
        input: String,
    ) -> Result<()> {
        let body = json!({
            "pending_state_num": pending_state_num,
            "init_num_batch": init_num_batch,
            "final_new_batch": final_new_batch,
            "new_local_exit_root": format!("0x{}", hex::encode(new_local_exit_root)),
            "new_state_root": format!("0x{}", hex::encode(new_state_root)),
            "proof": proof,
            "input": input
        });

        let response = self
            .client
            .post(format!(
                "{}/verify-batches-trusted-aggregator",
                self.url.clone()
            ))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(parsed_resp) => {
                            if let Some(status) = parsed_resp.get("status").and_then(|s| s.as_u64())
                            {
                                if status == SUCCESS_STATUS {
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "Contract call failed with status: {}",
                                        status
                                    ))
                                }
                            } else {
                                Err(anyhow::anyhow!("Response missing 'status' field"))
                            }
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to parse response: {:?}", e)),
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use self::hex::FromHex;
    use super::*;
    use crate::config::env::GLOBAL_ENV;
    use crate::settlement::custom::CustomSettlementConfig;
    use crate::settlement::Settlement;
    use crate::settlement::{init_settlement_provider, NetworkSpec};
    use anyhow::anyhow;

    fn setup() -> Result<Box<dyn Settlement>> {
        let config = CustomSettlementConfig {
            service_url: GLOBAL_ENV.bridge_service_addr.clone(),
        };
        let settlement_spec = NetworkSpec::Custom(config);
        let settlement_provider = init_settlement_provider(settlement_spec)
            .map_err(|e| anyhow!("Failed to init settlement: {:?}", e))?;

        Ok(settlement_provider)
    }
    #[tokio::test]
    #[ignore = "slow"]
    async fn test_bridge_asset() {
        let settlement_provider = setup().unwrap();
        let res = settlement_provider
            .bridge_asset(
                1,
                Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap_or_default(),
                U256::from_str(
                    "0x0000000000000000000000000000000000000000000000008ac7230489e80000",
                )
                .unwrap_or_default(),
                Address::from_str("0x0165878A594ca255338adfa4d48449f69242Eb8F").unwrap_or_default(),
                true,
                Bytes::from_str("0x").unwrap_or_default(),
            )
            .await;
        println!("res: {:?}", res);
    }

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_claim_asset() {
        let settlement_provider = setup().unwrap();
        let hex_strings = [
            "0x47f0e805088e58e4275879f10276b07b70bf8a009a2e2dbbf3220cab30340fca",
            "0xad3228b676f7d3cd4284a5443f17f1962b36e491b30a40b2405849e597ba5fb5",
            "0xb4c11951957c6f8f642c4af61cd6b24640fec6dc7fc607ee8206a99e92410d30",
            "0x21ddb9a356815c3fac1026b6dec5df3124afbadb485c9ba5a3e3398a04b7ba85",
            "0xe58769b32a1beaf1ea27375a44095a0d1fb664ce2dd358e7fcbfb78c26a19344",
            "0x0eb01ebfc9ed27500cd4dfc979272d1f0913cc9f66540d7e8005811109e1cf2d",
            "0x887c22bd8750d34016ac3c66b5ff102dacdd73f6b014e710b51e8022af9a1968",
            "0xffd70157e48063fc33c97a050f7f640233bf646cc98d9524c6b92bcf3ab56f83",
            "0x9867cc5f7f196b93bae1e27e6320742445d290f2263827498b54fec539f756af",
            "0xcefad4e508c098b9a7e1d8feb19955fb02ba9675585078710969d3440f5054e0",
            "0xf9dc3e7fe016e050eff260334f18a5d4fe391d82092319f5964f2e2eb7c1c3a5",
            "0xf8b13a49e282f609c317a833fb8d976d11517c571d1221a265d25af778ecf892",
            "0x3490c6ceeb450aecdc82e28293031d10c7d73bf85e57bf041a97360aa2c5d99c",
            "0xc1df82d9c4b87413eae2ef048f94b4d3554cea73d92b0f7af96e0271c691e2bb",
            "0x5c67add7c6caf302256adedf7ab114da0acfe870d449a3a489f781d659e8becc",
            "0xda7bce9f4e8618b6bd2f4132ce798cdc7a60e7e1460a7299e3c6342a579626d2",
            "0x2733e50f526ec2fa19a22b31e8ed50f23cd1fdf94c9154ed3a7609a2f1ff981f",
            "0xe1d3b5c807b281e4683cc6d6315cf95b9ade8641defcb32372f1c126e398ef7a",
            "0x5a2dce0a8a7f68bb74560f8f71837c2c2ebbcbf7fffb42ae1896f13f7c7479a0",
            "0xb46a28b6f55540f89444f63de0378e3d121be09e06cc9ded1c20e65876d36aa0",
            "0xc65e9645644786b620e2dd2ad648ddfcbf4a7e5b1a3a4ecfe7f64667a3f0b7e2",
            "0xf4418588ed35a2458cffeb39b93d26f18d2ab13bdce6aee58e7b99359ec2dfd9",
            "0x5a9c16dc00d6ef18b7933a6f8dc65ccb55667138776f7dea101070dc8796e377",
            "0x4df84f40ae0c8229d0d6069e5c8f39a7c299677a09d367fc7b05e3bc380ee652",
            "0xcdc72595f74c7b1043d0e1ffbab734648c838dfb0527d971b602bc216c9619ef",
            "0x0abf5ac974a1ed57f4050aa510dd9c74f508277b39d7973bb2dfccc5eeb0618d",
            "0xb8cd74046ff337f0a7bf2c8e03e10f642c1886798d71806ab1e888d9e5ee87d0",
            "0x838c5655cb21c6cb83313b5a631175dff4963772cce9108188b34ac87c81c41e",
            "0x662ee4dd2dd7b2bc707961b1e646c4047669dcb6584f0d8d770daf5d7e7deb2e",
            "0x388ab20e2573d171a88108e79d820e98f26c0b84aa8b2f4aa4968dbb818ea322",
            "0x93237c50ba75ee485f4c22adf2f741400bdf8d6a9cc7df7ecae576221665d735",
            "0x8448818bb4ae4562849e949e17ac16e0be16688e156b5cf15e098c627c0056a9",
        ];
        let mut smt_proof: [[u8; 32]; 32] = Default::default();
        for (i, hex_str) in hex_strings.iter().enumerate() {
            let hex_str = hex_str.trim_start_matches("0x");
            let bytes = <Vec<u8> as FromHex>::from_hex(hex_str).unwrap_or_default();
            smt_proof[i].copy_from_slice(&bytes);
        }
        let res = settlement_provider.claim_asset(
            smt_proof,
            1,
            <[u8; 32]>::from_hex("0xaadca94ab223b3e975c1cdfed7e2248ab7d91f056ba8f6d9060a36799f950a7e".trim_start_matches("0x")).expect("Invalid hex string"),
            <[u8; 32]>::from_hex("0xaadca94ab223b3e975c1cdfed7e2248ab7d91f056ba8f6d9060a36799f950a7e".trim_start_matches("0x")).expect("Invalid hex string"),
            0,
            Address::from_str("0x0165878A594ca255338adfa4d48449f69242Eb8F").unwrap_or_default(),
            1,
            Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap_or_default(),
            U256::from_str("0x0000000000000000000000000000000000000000000000008ac7230489e80000").unwrap_or_default(),
            Bytes::from_str("0x000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000000b456967656e20546f6b656e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005454947454e000000000000000000000000000000000000000000000000000000").unwrap_or_default(),
        ).await;

        println!("res: {:?}", res);
    }
    #[tokio::test]
    #[ignore = "slow"]
    async fn test_get_global_exit_root() {
        let settlement_provider = setup().unwrap();
        let res = settlement_provider.get_global_exit_root().await;
        println!("res: {:?}", res);
    }

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_sequence_batches() {
        let settlement_provider = setup().unwrap();
        let batches = vec![BatchData {
            transactions: vec![1, 2, 3, 4, 5],
            global_exit_root: <[u8; 32]>::from_hex(
                "0xaadca94ab223b3e975c1cdfed7e2248ab7d91f056ba8f6d9060a36799f950a7e"
                    .trim_start_matches("0x"),
            )
            .expect("Invalid hex string"),
            timestamp: 1634073200,
        }];

        let res = settlement_provider.sequence_batches(batches).await;
        println!("res: {:?}", res);
    }

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_batch_struct() {
        let batches = vec![BatchData {
            transactions: vec![1, 2, 3, 4, 5],
            global_exit_root: <[u8; 32]>::from_hex(
                "0xaadca94ab223b3e975c1cdfed7e2248ab7d91f056ba8f6d9060a36799f950a7e"
                    .trim_start_matches("0x"),
            )
            .expect("Invalid hex string"),
            timestamp: 1634073200,
        }];

        let batches_json: Vec<_> = batches
            .clone()
            .into_iter()
            .map(|batch| {
                json!({
                    "transactions": format!("0x{}", hex::encode(batch.transactions)),
                    "global_exit_root": format!("0x{}", hex::encode(batch.global_exit_root)),
                    "timestamp": batch.timestamp
                })
            })
            .collect();

        println!("batches: {:?}", batches_json);

        #[derive(Debug)]
        pub struct SolidityBatchData {
            pub transactions: Bytes,
            pub global_exit_root: [u8; 32],
            pub timestamp: u64,
        }

        let solidity_batches: Vec<SolidityBatchData> = batches
            .iter()
            .map(|b| SolidityBatchData {
                transactions: Bytes::from(b.transactions.clone()),
                global_exit_root: b.global_exit_root,
                timestamp: b.timestamp,
            })
            .collect();

        println!("solidity_batches: {:?}", solidity_batches);
    }

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_verify_batches() {
        let settlement_provider = setup().unwrap();
        let res = settlement_provider.verify_batches(
            0,
            0,
            1,
            <[u8; 32]>::from_hex("0xaadca94ab223b3e975c1cdfed7e2248ab7d91f056ba8f6d9060a36799f950a7e".trim_start_matches("0x")).expect("Invalid hex string"),
            <[u8; 32]>::from_hex("0xaadca94ab223b3e975c1cdfed7e2248ab7d91f056ba8f6d9060a36799f950a7e".trim_start_matches("0x")).expect("Invalid hex string"),
            "{\"pi_a\":{\"x\":\"9898772573056133167685730031820439200779677251915533133089180906078340598153\",\"y\":\"4988585277003916341444053304584863613990493280622140059976827625686886539523\"},\"pi_b\":{\"x\":[\"20509617804647477524035568417927205406397155061230874893680539238495862881580\",\"6585671937830049132098354513544251392871629080955562897302125989145227322378\"],\"y\":[\"16600973708004378995096185526104442264785941566440480473110565973138301268148\",\"4748797935380146483285828556664955484346426282325689850322414929184461946949\"]},\"pi_c\":{\"x\":\"13887335001047178786869303446566979485698626327666217004092778514366245776228\",\"y\":\"14978410198885453128070499586881293125336387926725723435864499846753605937804\"},\"protocol\":\"groth16\",\"curve\":\"BN128\"}".to_string(),
            "[\"8362010813876620668586816087691982027748700171077865203929785134913673568689\"]".to_string(),
        ).await;
        println!("res: {:?}", res);
    }

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_update_exit_root() {
        let settlement_provider = setup().unwrap();
        // only bridgeAddress or rollupAddress can call
        let res = settlement_provider
            .update_exit_root(
                1,
                <[u8; 32]>::from_hex(
                    "0xaadca94ab223b3e975c1cdfed7e2248ab7d91f056ba8f6d9060a36799f910a7e"
                        .trim_start_matches("0x"),
                )
                .expect("Invalid hex string"),
            )
            .await;
        println!("res: {:?}", res);
    }
}

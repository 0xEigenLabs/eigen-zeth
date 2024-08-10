CARGO = cargo

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
	CARGO += --config 'build.rustdocflags = ["-C", "link-args=-framework CoreFoundation -framework Security"]'
endif

help: ## Display this help screen
	@grep -h \
		-E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

clippy: ## Run clippy checks over all workspace members and formatted correctly
	@cargo check
	@cargo fmt --all -- --check
	@cargo clippy --all-targets -- -D warnings --no-deps

fix: ## Automatically apply lint suggestions. This flag implies `--no-deps` and `--all-targets`
	@cargo clippy --fix

test: ## Run tests for all the workspace members
	@cargo test --release --all

install: ## Install the binary for eigen-zeth project using cargo
	@cargo install --path . --force

generate_genesis_data: ## Generate genesis data
	@cd scripts && sh ./generate-genesis-data.sh

build_eth2_validator_tools: ## Build eth2 validator tools
	@cd scripts && sh ./build-eth2-validator-tools.sh

generate_eth2_validator_keys: ## Generate eth2 validator keys
	@cd scripts && sh ./generate-eth2-validator-keys.sh

init_eigen_zeth_genesis: ## Initialize eigen zeth genesis
	@cd scripts && sh ./init-eigen-zeth-genesis.sh

launch_pos_eigen_zeth_node: ## Launch a single eigen_zeth node using PoS consensus algorithm
	@cd scripts && sh ./launch-pos-eigen-zeth-node.sh

stop_pos_eigen_zeth_node: ## Stop eigen zeth node
	@pkill lighthouse
	@pkill eigen-zeth

clean_pos_eigen_zeth_network_data: ## Clean eigen zeth network data
	@rm -rf tmp/layer2/consensus-data
	@rm -rf tmp/layer2/execution-data
	@rm -rf tmp/operator
	@rm tmp/zeth.log
	@rm tmp/beacon.log
	@rm tmp/validator.log

build_lighthouse: ## Build and install lighthouse
	@cd scripts && sh ./build-and-install-lighthouse.sh

.PHONY: clippy fmt test

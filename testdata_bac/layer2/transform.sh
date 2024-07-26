#!/bin/bash

genesis_json="chain.json"
lighthouse_config="lighthouse-genesis.toml"

# 脚本示例，将 Genesis JSON 转换为 Lighthouse 创世配置的 TOML 格式
cat <<EOF > "$lighthouse_config"
# Lighthouse 创世配置文件

# 主网配置示例，请根据实际情况调整
[eth2]
# 节点类型（信标节点）
type = "beacon"

# 主网络网络ID
network_id = "1"

# 信标链的配置信息
[eth2.genesis]
# 使用 Lighthouse 的 Genesis 转换的信息
genesis = "$(cat $genesis_json)"

EOF


# DePINC Bridge 桥服务

提供 DePC 与 Solana Spl-token 相互转换的桥梁

## 模块划分

### Bridge

用于监视 DePINC 链上交易，然后提取出 Deposit 和 Withdrawal，继而在交易被确认后，分别向 Solana 和 DePINC 链上发起对应的交易，当交易完成后向 DePINC 链上发起特殊交易用于补充 Solana 链上信息。

* 监视 DePINC 链上交易分离出：Deposit 完成请求和 Withdrawal 完成请求

**Deposit**

* 在 Solana 链上发起交易，将 Spl-token 转至指定的账户

* 然后在 DePINC 链上发起新的交易，将 Solana 链上新的交易的 signature 保存到 DePINC 链上

**Withdrawal**

* 扫描到用户在 DePINC 链上发起的请求交易，将下面项目从请求交易中取出：
  1. 对应在 Solana 链上的 signature
  2. 目标 DePC 地址

* 检查 Solana 链上的 signature 已确认

* 在 DePINC 链上发起新的交易，将对应金额的 DePC 转入到目标 DePC 地址，同时该交易还需要包含 Solana 上的 signature

DePINC 链上需要创建以下几种特殊交易：

用户发起的交易：

1. Deposit

2. Withdrawal request

桥在后续操作完成后将补完 DePC 链上交易：

1. Deposit finalize

2. Withdraw transaction

### DePC

1. DePINC 链上相关的交易提取

2. 发起 DePINC 链上特殊交易

### Solana

1. Solana 链上交易的确认

2. 发起 Solana 链上 Spl-token 交易

### Local database

本地必要的数据的存储支持

### Rest

提供必要的 http API

### RPC

暂时只被 DePC 模块使用，与 DePINC 节点通信

# DePINC Bridge 桥服务

提供 DePC 与 Solana Spl-token 相互转换的桥梁。

Bridge 监视 DePINC 链上交易，然后提取出 Deposit 和 Withdrawal 相关的特殊交易，在 Solana 链上发起 Tpl-token 交易以及校验用户发起的在 Solana 上的 Tpl-token 交易。在用户的交易被确认后，分别向 Solana 和 DePINC 链上发起对应的 Deposit 和 Withdrawal 交易，当交易完成后向 DePINC 链上发起特殊交易用于补充 Solana 链上信息。

## Deposit 和 Withdrawal 交易类型和步骤

**Deposit**

Deposit 是指：用户将 DePC 代币从 DePINC 网络转移至 Solana 网络。

1. 用户在 DePINC 链上，向机构的地址发起包含一个 Solana 地址的特殊交易，同时向机构存入一定额度的 DePC 代币；

2. Bridge 扫描到该交易后，会在 Solana 网络上将该额度（可能扣除一定手续费）的 DePC 代币存入用户指定地址的 Spl-token 账户；

3. 当该条交易在 Solana 上被确认后，Bridge 会在 DePINC 网络上使用步骤 1 中的特殊交易的 Txout 作为 Input 发起一笔新的特殊交易，同时把在 Solana 上被确认的这条交易的 Signature 保存到这条新的 DePINC 的交易里。

**Withdrawal**

Withdrawal 是指：用户将 DePC 代币从 Solana 网络转移至 DePINC 网络。

1. 用户在 Solana 链上发起交易，将 DePC 代币转至机构的 Spl-token 地址；

2. 然后用户在 DePINC 链上发起一条特殊的请求交易，将 Solana 链上新的交易的 signature 和 Withdrawal 的目标地址（DePINC 链上）附加到该交易上；

3. Bridge 扫描 DePINC 网络发现该请求交易，然后获取用户提供的 Solana 的交易 Signature；

4. Bridge 在 Solana 网络上验证该条交易是否已经被确认，并且该条交易在 DePINC 网络上并没有对应的 Withdrawal 交易；

5. 确认了在 Solana 网络上的交易的有效性后，Bridge 在 DePINC 网络上获取这条交易的目标地址和交易额度，然后发起一条新的特殊交易。该交易使用步骤 2 中的 Txout 作为 Input，同时将该额度（可能扣除一定手续费）的 DePC 转至用户指定的地址；

## 模块划分

### Bridge

主要的交易的执行逻辑，包括创建 Channel，调用子模块获取 DePINC 链上信息，本地数据库的保存操作，多线程数据传递以及 Solana 链上交易确认和发起。

### DePC

DePINC 链上操作的基础模块，包含扫描，发起交易等。

1. DePINC 链上相关的交易提取；

2. 发起 DePINC 链上特殊交易；

### Solana

Solana 链上的操作基础模块，包含交易确认，发起交易等。

1. Solana 链上交易的确认；

2. 发起 Solana 链上 Spl-token 交易；

### Local database

本地必要的数据的存储支持，暂时使用 SQLite3 作为主数据存储引擎。

### Rest

提供必要的 http API，具体接口待定，但包含以下两大类：

1. 提供给客户端用户使用；

2. 提供给后台管理系统使用；

### RPC

暂时只被 DePC 模块使用，与 DePINC 节点通信。

## 编译与安装

TODO

## 配置和参数说明

TODO

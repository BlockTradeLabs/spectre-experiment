## Pallet Spectre

### Extrinsics
 - **register_investor**
 
    This extrinsic registers investor by depositing capital to the pool and registering the details in `InvestorProfile`
    As the Investor can register in atmost 4 pools supporting assets
    Calling this function by specifying the asset registers to the specific pool, if the investors did register, it will add the assets pool balance.

- **register_trader**

    Registers trader after generating on chain trading accounts in the contract.The details are registered in `TraderProfile`
    This extrinsic accept the trading acconts public key to registers them with trader account id

- **allocate_capital**(Not implemented yet)

    Allocate capital from the pool to the trader onchain trading account

- **verify_trade_execution**

    Verify trade executed in the foreigh Dex signed by trader onchain trading account
    This extrinsics accepts `TradeExecutionProof` and `TradeAction` sepcifying the type of trade and proofs neccessary for verification

### Storage

- **InvestorProfiles**

    Storing Investor registered profiles,
    
    StorageMap
    `AccountId` -> `InvestorProfile`
- **TraderProfiles**

    Storing Trader registered profiles,

    StorageDoubleMap (2 keys )
    `AccountId` & `OnchainTradingAccountPublicKey` -> `TraderProfile`
- **OnChainTradingAccounts**

    Storing trader's onchain trading accounts public key generated in the contract. Note that the contract stores the associated private keys opaqely.

    StorageMap
    `AccountId` -> `TradingAccount`   
- **CapitalPool**

    Storing investor's contributed capital and keep tracks of changes in balance per trading activities. There are different pools per asset id.

    StorageMap
    `CurrencyId` ->  `InvestorCapitalPool`
- **Relayer**

    Storing account responsible for signing trader registration transactions. This account is the same as the one in the contract stored.
    The storage is set on genesis




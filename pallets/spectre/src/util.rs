#![cfg_attr(not(feature = "std"), no_std)]

use {
    frame_support::{pallet_prelude::*, DefaultNoBound},
    frame_system::pallet_prelude::*,
    sp_io::hashing::blake2_128,
    sp_std::{vec, vec::Vec},
};

use super::pallet::*;

pub use utils::*;

pub mod utils {

    extern crate alloc;

    use {
        frame_support::sp_runtime::traits::BlakeTwo256,
        sp_core::{
            serde::{Deserialize, Serialize},
            H256,
        },
        sp_trie::{LayoutV1, StorageProof, TrieDBBuilder},
    };

    use {
        alloc::collections::BTreeMap,
        frame_support::sp_runtime::{traits::TrailingZeroInput, MultiAddress},
        sp_arithmetic::Permill,
    };
    // use sp_core::{blake2_128, ConstU8};
    use {
        //hash_db::HashDB,
        parity_scale_codec::{Decode, Encode},
        sp_core::ConstU8,
        sp_trie::Trie,
    };

    use super::*;

    impl<T: Config> Pallet<T> {
        // helper function to generate onchain keyless account
        pub fn generate_pool_account(asset_id: T::CurrencyId) -> AccountIdFor<T> {
            let entropy = (b"spectre/salt", asset_id).using_encoded(blake2_128);
            let pool_account_id = Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
                .expect("Infinite length input, Cant create an account");

            pool_account_id
        }

        // For trade execution verifier
        // pub fn read_proof_check<H, I>(
        //     root: &H::Out,
        //     proof: StorageProof,
        //     keys: I,
        // ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error<T>>
        // where
        //     H: hash_db::Hasher,
        //     H::Out: scale_info::prelude::fmt::Debug,
        //     I: IntoIterator,
        //     I::Item: AsRef<[u8]>,
        // {
        //     let db = proof.into_memory_db();

        //     if !db.contains(root, hash_db::EMPTY_PREFIX) {
        //         Err(Error::<T>::FailedTradeProof)?
        //     }

        //     let trie = TrieDBBuilder::<LayoutV1<H>>::new(&db, root).build();
        //     let mut result = BTreeMap::new();

        //     for key in keys.into_iter() {
        //         let value = trie
        //             .get(key.as_ref())
        //             .map_err(|e| Error::<T>::FailedTradeProof)?
        //             .and_then(|val| Decode::decode(&mut &val[..]).ok());
        //         result.insert(key.as_ref().to_vec(), value);
        //     }

        //     Ok(result)
        // }
    }

    /// Tracking Trader activities
    /// `trading account`: The linked on chain trading account per trader sovereign account
    /// `bonded amount`: Amount placed into hold by the trader signifying conviction
    /// `funds allocated`: Total amount allocated to trader from pool
    /// `credits`: Metrics to measure trader performance
    #[derive(Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct TraderProfile<T: Config> {
        pub trading_account: AccountIdFor<T>,
        pub asset_id: T::CurrencyId,
        pub bonded_amount: TraderBond<T>,
        pub funds_allocated: AssetBalance<T>, //BalanceOf<T>,
        pub unrealized_balance: AssetBalance<T>,
        pub credits: u8,
        pub trades_executed: u16,
    }

    impl<T: Config> TraderProfile<T> {
        pub fn update_unrealized_balance(&mut self, balance: AssetBalance<T>) {
            self.unrealized_balance = balance;
            self.trades_executed += 1
        }

        pub fn deposit_allocated_funds(&mut self, balance: AssetBalance<T>) {
            self.funds_allocated += balance
        }

        pub fn new(asset_id: T::CurrencyId, trading_account: AccountIdFor<T>) -> Self {
            // needs to calculate credits upon registering new trader
            // TODO!!!!
            Self {
                trading_account,
                asset_id,
                bonded_amount: TraderBond::<T>::default(),
                funds_allocated: AssetBalance::<T>::default(),
                unrealized_balance: AssetBalance::<T>::default(),
                credits: 0,
                trades_executed: 0,
            }
        }
    }

    /// Tracking investor investments
    /// `deposited_capital`: Total capital deposited/ contributed to the pool
    /// `lp_ownership`: Total pool percentage ownerhip per ownership
    /// `accumulated profit`: Total points representing profits to be later claimed
    /// `withdraw_period`: Total time that should elapse for investor to withdraw capital + profit
    #[derive(Encode, Decode, Clone, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct InvestorProfile<T: Config> {
        pub investor_id: Option<AccountIdFor<T>>,
        pub deposited_capital: Vec<(T::CurrencyId, AssetBalance<T>)>, //This should be BoundedBTreeMap but am getting lots of errors TODO! consider fixing
        //pub lp_ownership: Permill,
        pub block_number: BlockNumberFor<T>,
        pub claimed_profit: u32,
        pub withdraw_period: BlockNumberFor<T>,
    }

    impl<T: Config> Default for InvestorProfile<T> {
        fn default() -> Self {
            Self {
                deposited_capital: vec![],
                block_number: <frame_system::Pallet<T>>::block_number(),
                claimed_profit: 0,
                withdraw_period: T::WithdrawPeriod::get(),
                investor_id: None,
            }
        }
    }

    impl<T: Config> InvestorProfile<T> {
        pub fn register_capital(
            &mut self,
            investor_id: AccountIdFor<T>,
            asset_id: T::CurrencyId,
            amount: AssetBalance<T>,
        ) {
            self.investor_id = Some(investor_id);
            // check if the capital under the asset has been already provided
            self.deposited_capital
                .clone()
                .iter()
                .for_each(|(inner_asset_id, mut balance)| {
                    if &asset_id == inner_asset_id {
                        balance += amount
                    } else {
                        self.deposited_capital.push((asset_id.clone(), amount))
                    }
                });
        }
    }

    /// Capital Pool management
    /// `total_capital`: Total contributed asset amount
    /// `remaining_capital`: Total capital after allocation
    /// `total_allocated_capital`: Total allocated funds to traders
    /// `unrealized_balance`: `total_capital` + profits after trades
    #[derive(Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct InvestorCapitalPool<T: Config> {
        pub asset_name: T::CurrencyId,
        pub total_capital: AssetBalance<T>, //BalanceOf<T>,
        pub remaining_capital: AssetBalance<T>,
        pub total_allocated_capital: AssetBalance<T>,
        pub unrealized_balance: AssetBalance<T>,
        pub fee: u8, // in percentage,
        pub account_id: AccountIdFor<T>,
    }

    impl<T: Config> InvestorCapitalPool<T> {
        pub fn update_allocated_funds(&mut self, amount: AssetBalance<T>) {
            self.remaining_capital -= amount;
            self.total_allocated_capital += amount
        }

        pub fn add_capital(&mut self, amount: AssetBalance<T>) {
            self.total_capital += amount;
            self.remaining_capital += amount;
        }

        pub fn deduct_unreliazed_balance(&mut self, amount: AssetBalance<T>) {
            self.unrealized_balance -= amount
        }

        pub fn add_unrealized_balance(&mut self, amount: AssetBalance<T>) {
            self.unrealized_balance += amount
        }
    }

    impl<T: Config> Default for InvestorCapitalPool<T> {
        fn default() -> InvestorCapitalPool<T> {
            let account_id = Pallet::<T>::generate_pool_account(T::CurrencyId::default());
            Self {
                asset_name: T::CurrencyId::default(),
                total_capital: AssetBalance::<T>::default(),
                fee: 1,
                remaining_capital: AssetBalance::<T>::default(),
                total_allocated_capital: AssetBalance::<T>::default(),
                unrealized_balance: AssetBalance::<T>::default(),
                account_id,
            }
        }
    }

    /// Trader bond details and indicator if the bond should be staked for more rewards
    #[derive(Encode, Decode, Clone, DefaultNoBound, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct TraderBond<T: Config> {
        pub amount: T::CurrencyId,
        pub stake: bool,
    }

    #[derive(
        Encode, Decode, Clone, PartialEq, RuntimeDebug, DefaultNoBound, MaxEncodedLen, TypeInfo,
    )]
    pub struct TradingAccounts<AccountId> {
        pub substrate: Option<AccountId>,
        pub ethereum: Option<AccountId>,
        pub solana: Option<AccountId>,
    }

    /// Hashing algorithm for the state proof
    // #[derive(Debug, Encode, Decode, Clone, Serialize, Deserialize)]
    // pub enum HashAlgorithm {
    //     /// For chains that use keccak as their hashing algo
    //     Keccak,
    //     /// For chains that use blake2 as their hashing algo
    //     Blake2,
    // }

    /// Holds the relevant data needed for state proof verification
    // #[derive(Debug, Encode, Decode, Clone)]
    // pub struct SubstrateStateProof {
    //     /// Algorithm to use for state proof verification
    //     pub hasher: HashAlgorithm,
    //     /// Storage proof for the parachain headers
    //     pub storage_proof: Vec<Vec<u8>>,
    // }

    #[derive(Debug, Encode, Decode, Clone)]
    pub enum SupportedDexs {
        HydraDx,
        StellaSwap,
        EthUniswap,
        PolyUniswap,
        ArbUniswap,
        Jupiter,
        BaseUniswap,
        BnbUniswap,
    }

    /// This object is responsible for verifying and proving trade execution done in another consensus network
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct TradeExecutionProof<BlockNumber> {
        pub target_network: Networks,
        pub target_network_blocknumber: BlockNumber,
        pub transaction_inclusion: TransactionInclusionProof,
        pub state_proof: StateProof,
        pub consensus_proof: Option<ConsensusProofs>,
    }

    /// Data to verify inclusion of the trade transaction
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct TransactionInclusionProof {
        tx_id: Vec<u8>,
        tx_proof: Vec<Vec<u8>>,
        key: Vec<u8>,
        tx_state_root: Vec<u8>,
    }

    /// Data to verify and read account balance after trade transaction
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct StateProof {
        pub state_root: Vec<u8>,
        pub state_proofs: Vec<Vec<u8>>,
        pub state_key: Vec<u8>,
    }

    /// Data to verify the canonical state of the target state machine
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct ConsensusProofs {
        pub consensus_root: Vec<u8>,
        pub consensus_proofs: Vec<Vec<u8>>,
        pub consensus_digest: Vec<u8>,
        pub consensus_digest_key: Vec<u8>,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    pub enum Networks {
        Substrate,
        Ethereum,
        Solana,
        Sei,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    pub enum TradeAction {
        Buy,
        Sell,
    }

    // Traits and implementations

    /// Responsible for allocating funds from different pools to trader on chain trading account
    pub trait CapitalAllocator<T: Config> {
        fn allocate_capital(
            network: Networks,
            trader_id: AccountIdFor<T>,
            onchain_trading_account: AccountIdFor<T>,
        ) -> DispatchResult;
    }

    impl<T: Config> CapitalAllocator<T> for () {
        fn allocate_capital(
            network: Networks,
            trader_id: AccountIdFor<T>,
            onchain_trading_account: AccountIdFor<T>,
        ) -> DispatchResult {
            Ok(())
        }
    }

    /// Responsible for claiming Return on Investment by Investor and trader
    pub trait Withdraw<T: Config> {
        fn investor_instant_withdraw() -> DispatchResult;

        fn investor_schedule_withdraw() -> DispatchResult;

        fn trader_instant_withdraw() -> DispatchResult;

        fn trader_schedule_withdraw() -> DispatchResult;
    }

    /// Responsible for verifying trade execution proofs

    pub trait TradeExecutionVerifier<T: Config> {
        // Verify Trade execution in a foreign Dex in a target network
        fn verify_trade_execution(
            trader_id: AccountIdFor<T>,
            trading_account: AccountIdFor<T>,
            asset_id: T::CurrencyId,
            network: Networks,
            proofs: TradeExecutionProof<BlockNumberFor<T>>,
            trade_action: TradeAction,
        ) -> DispatchResult;

        // Verify trade transaction inclusion in the block of the target network ( Blockchain )
        fn verify_trade_tx_inclusion(network: Networks, proofs: TransactionInclusionProof) -> bool;

        // Verify state proofs and read the account balance
        fn verify_state_acount_balance(
            network: Networks,
            proofs: StateProof,
        ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error<T>>;

        // Verify consensus commitment on N blockheight
        fn verify_consensus_state(network: Networks, proofs: ConsensusProofs) -> bool;
    }

    pub struct TradeExecutionVerifyV1;

    impl<T: Config> TradeExecutionVerifier<T> for TradeExecutionVerifyV1 {
        fn verify_trade_execution(
            trader_id: AccountIdFor<T>,
            trading_account: AccountIdFor<T>,
            asset_id: T::CurrencyId,
            network: Networks,
            proofs: TradeExecutionProof<BlockNumberFor<T>>,
            trade_action: TradeAction,
        ) -> DispatchResult {
            // This will be crucial once targeting other non shared security chains
            // let is_consensus_valid = T::TradeExecutionVerifier::verify_consensus_state(
            //     network.clone(),
            //     proofs.consensus_proof,
            // );

            let is_tx_valid = T::TradeExecutionVerifier::verify_trade_tx_inclusion(
                network.clone(),
                proofs.transaction_inclusion,
            );

            if !is_tx_valid {
                return Err(Error::<T>::InvalidTxInclusion.into()); // The Tx was not found
            }

            let state_account_balance =
                T::TradeExecutionVerifier::verify_state_acount_balance(network, proofs.state_proof)
                    .map_err(|_| {
                        Error::<T>::InvalidBalanceStateProof
                    })?;

            // update pool and trader balance
            ensure!(
                CapitalPool::<T>::contains_key(asset_id),
                Error::<T>::AssetPoolNotSupported
            );

            // check the asset id and fetch the associated account id for trader

            let mut trader_profile = TraderProfiles::<T>::get(trader_id, trading_account.clone())
                .ok_or(Error::<T>::TraderNotFunded)?;

            let capital_pool = CapitalPool::<T>::get(asset_id);

            // get the trader trading balance remaining in on chain trading account
            let trading_acc_vec = trading_account.encode();
            let rem_trading_balance_encoded = state_account_balance
                .get(&trading_acc_vec)
                .ok_or(Error::<T>::InvalidBalanceStateProof)?
                .clone()
                .ok_or(Error::<T>::InvalidBalanceStateProof)?;

            let rem_trading_balance: AssetBalance<T> =
                Decode::decode(&mut &rem_trading_balance_encoded[..]).map_err(|_| {
                    Error::<T>::FailedToDecodeValue
                })?;

            // get the net positive or negative balance
            // let loss = allocated_balance  - rem_trading_balance;
            // let profit = rem_trading_balance - allocated_balance;

            match trade_action {
                TradeAction::Buy => {
                    trader_profile.update_unrealized_balance(rem_trading_balance);
                    // update the pool
                }
                TradeAction::Sell => {
                    trader_profile.update_unrealized_balance(rem_trading_balance);
                    // update the pool
                }
            }

            Ok(())
        }

        fn verify_consensus_state(network: Networks, proofs: ConsensusProofs) -> bool {
            true
        }

        fn verify_state_acount_balance(
            network: Networks,
            proofs: StateProof,
        ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error<T>> {
            match network {
                Networks::Substrate => {
                    let data = {
                        let db =
                            StorageProof::new(proofs.state_proofs).into_memory_db::<BlakeTwo256>();

                        let state_proof_root = H256::from_slice(&proofs.state_root[..]);

                        let trie =
                            TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&db, &state_proof_root)
                                .build();

                        vec![proofs.state_key]
                            .into_iter()
                            .map(|key| {
                                let value = trie.get(&key).map_err(|e| Error::FailedTradeProof)?;
                                Ok((key, value))
                            })
                            .collect::<Result<BTreeMap<_, _>, _>>()?
                    };

                    Ok(data)
                }
                _ => todo!(),
            }
        }

        fn verify_trade_tx_inclusion(network: Networks, proofs: TransactionInclusionProof) -> bool {
            match network {
                Networks::Substrate => {
                    let tx_root = H256::from_slice(&proofs.tx_state_root[..]);
                    let is_valid = sp_trie::verify_trie_proof::<
                        sp_trie::LayoutV1<BlakeTwo256>,
                        _,
                        Vec<u8>,
                        Vec<u8>,
                    >(
                        &tx_root,
                        &*proofs.tx_proof,
                        &[(proofs.key, Some(proofs.tx_id))],
                    );
                    if is_valid.is_ok() {
                        true
                    } else {
                        false
                    }
                }
                _ => todo!("Ethereum implementation"),
            }
        }
    }

    // Reference Implementation
    // match network {
    //     Networks::Substrate => {

    //         let extrinsic_root = H256::from_slice(&trade_execution_proof.extrinsic_root);
    //         let extrinsics_proof_nodes = &trade_execution_proof.extrinsic_proofs;
    //         let extrinsic_data = &trade_execution_proof.extrinsic_data;
    //         let extrinsic_key = &trade_execution_proof.extrinsic_key;
    //         // state data
    //         let state_root = H256::from_slice(&trade_execution_proof.state_root);
    //         let state_proof_nodes = &trade_execution_proof.state_proofs;
    //         let state_key = &trade_execution_proof.state_key;

    //          // verify extrinsic inclusion
    //         if let Err(_extrinsic_proof_error) =
    //         sp_trie::verify_trie_proof:: <sp_trie::LayoutV1<BlakeTwo256> ,_,Vec<u8> ,Vec<u8> >(
    //             &extrinsic_root,
    //             &*extrinsics_proof_nodes.to_vec(),
    //             &[(extrinsic_key.to_vec(), Some(extrinsic_data.to_vec()))],
    //         )
    //         {
    //             return Err(TransactionValidityError::Unknown(UnknownTransaction::Custom(1))); // 1 for extrinsic verification error
    //         }

    //         // verify state change
    //         // I think we dont need to do state verification as we will be just fetching the value at the end of the day manually from the proofs
    //         // if let Err(_state_proof_error) =
    //         // verify_trie_proof::<LayoutV1<BlakeTwo256>, _, Vec<u8>, Vec<u8>>(
    //         //     &state_root,
    //         //     &*state_proof_nodes.to_vec(),
    //         //     &[(state_key.to_vec(), None)],
    //         // )
    //         // {
    //         //     return Err(TransactionValidityError::Unknown(UnknownTransaction::Custom(2))); // 2 for state verification error
    //         // }

    //          // get the balance data from state data
    //         let database = StorageProof::new(state_proof_nodes.to_vec()).to_memory_db::<BlakeTwo256>();
    //         let encoded_balance = read_trie_value::<LayoutV1<BlakeTwo256>, _>(
    //             &database,
    //             &state_root,
    //             &state_key,
    //             None,
    //             None,
    //         )
    //         .map_err(|_| TransactionValidityError::Unknown(UnknownTransaction::Custom(3)))?
    //         .ok_or(TransactionValidityError::Unknown(UnknownTransaction::Custom(3)))?;

    //         let trading_roi: AssetBalance<T> /*This should asset id type */ = Decode::decode(&mut &encoded_balance[..])
    //             .map_err(|_| TransactionValidityError::Unknown(UnknownTransaction::Custom(4)))?;

    //         // reward algorithm
    //         //T::RewardDistribution::distribute_roi(network,trader_id);

    //         // Modify this to be dynamic in terms of priority,
    //         // All polkadot related verification should have lesser priorioty than non polkadot trade verification
    //         Ok(ValidTransaction{
    //             priority: u64::MAX,
    //             requires: vec![],
    //             provides: vec![],
    //             longevity: TransactionLongevity::MAX,
    //             propagate: true,
    //         })
    //     },
    //     _ => {
    //         Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
    //     }
}

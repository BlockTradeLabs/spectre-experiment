#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::DefaultNoBound;
use frame_system::pallet_prelude::*;
use sp_std::vec;
use sp_std::vec::Vec;

use super::pallet::*;

pub use utils::*;

pub mod utils {

    use std::net::UdpSocket;

    extern crate alloc;

    use alloc::collections::BTreeMap;
    use frame_support::sp_runtime::{traits::TrailingZeroInput, MultiAddress};
    use sp_arithmetic::Permill;
    use sp_core::{blake2_128, ConstU8};
    use parity_scale_codec::{Encode,Decode};

    use super::*;

    impl<T: Config> Pallet<T> {
        // helper function to generate onchain keyless account 
        pub fn generate_pool_account(asset_id: T::AssetId) -> AccountIdFor<T> {
            let entropy = (b"spectre/salt",asset_id).using_encoded(blake2_128);
            let pool_account_id = Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
            .expect("Infinite length input, Cant create an account");

            pool_account_id
        }
    } 

    /// Tracking Trader activities
    /// `trading account`: The linked on chain trading account per trader sovereign account
    /// `bonded amount`: Amount placed into hold by the trader signifying conviction
    /// `funds allocated`: Total amount allocated to trader from pool
    /// `credits`: Metrics to measure trader performance
    #[derive(Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct TraderProfile<T: Config> {
        pub trading_account: Option<AccountIdFor<T>>,
        pub bonded_amount: TraderBond<T>,
        pub funds_allocated: u128, //BalanceOf<T>,
        pub credits: u8,
        pub trades_executed: u16,
    } 

    /// Tracking investor investments
    /// `deposited_capital`: Total capital deposited/ contributed to the pool
    /// `lp_ownership`: Total pool percentage ownerhip per ownership
    /// `accumulated profit`: Total points representing profits to be later claimed
    /// `withdraw_period`: Total time that should elapse for investor to withdraw capital + profit
    #[derive(Encode, Decode, Clone, Default, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct InvestorProfile<T: Config> {
        pub deposited_capital: Vec<(T::AssetId,u128)>, //This should be BoundedBTreeMap but am getting lots of errors TODO! consider fixing
        //pub lp_ownership: Permill,
        pub block_number: BlockNumberFor<T>,
        pub claimed_profit: u32,
        pub withdraw_period: BlockNumberFor<T>,
    }

    impl<T: Config> InvestorProfile<T> {
    

       pub fn add_capital(&mut self,asset_id:T::AssetId, amount:u128)-> DispatchResult{
            // check if the capital under the asset is alreayd provided
            // if let Some(existing_capital) = self.deposited_capital.get(asset_id){
            //     let updated_capital = existing_capital + amount;
            //     self.deposited_capital.try_insert(asset_id,updated_capital).map_err(|_|Error::<T>::FailedToAddCapital)?
            // }else{
            //     self.deposited_capital.try_insert(asset_id,amount).map_err(|_|Error::<T>::FailedToAddCapital)?
            // }
            // if let Some(existing_capital) = self.deposited_capital.get(&asset_id){
            //     let updated_capital = existing_capital.clone() + amount;
                
            // }else{

            // }
            Ok(())
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
        pub asset_name: T::AssetId,
        pub total_capital: u128, //BalanceOf<T>,
        pub remaining_capital: u128,
        pub total_allocated_capital: u128,
        pub unrealized_balance: u128,
        pub created_at: BlockNumberFor<T>,
        pub fee: u8, // in percentage, 
        pub account_id: AccountIdFor<T>
    }

    impl<T: Config> InvestorCapitalPool<T> {
        pub fn update_allocated_funds(&mut self, amount:u128) {
            self.remaining_capital -= amount;
            self.total_allocated_capital += amount
        }

        pub fn add_capital(&mut self, amount: u128) {
            self.total_capital += amount;
            self.remaining_capital += amount;
        }

        pub fn deduct_unreliazed_balance(&mut self, amount: u128) {
            self.unrealized_balance -= amount
        }

        pub fn add_unrealized_balance(&mut self, amount: u128){
            self.unrealized_balance += amount
        }
    }


    impl<T: Config> Default for InvestorCapitalPool<T> {
        fn default() -> InvestorCapitalPool<T> {

            let account_id = Pallet::<T>::generate_pool_account(T::DefaultAsset::get());
            let blocknumber =<frame_system::Pallet<T>>::block_number();
            Self {
                asset_name: T::DefaultAsset::get(), 
                total_capital: 0, 
                created_at: blocknumber, 
                fee: 1, 
                remaining_capital: 0, 
                total_allocated_capital: 0, 
                unrealized_balance: 0, 
                account_id
            }
        }
    }

    /// Trader bond details and indicator if the bond should be staked for more rewards
    #[derive(Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct TraderBond<T: Config> {
        pub amount: T::AssetId,
        pub stake: bool,
    }

    #[derive(
        Encode, Decode, Clone, PartialEq, RuntimeDebug, DefaultNoBound, MaxEncodedLen, TypeInfo,
    )]
    pub struct TradingAccounts<AccountId> {
        substrate: Option<AccountId>,
        ethereum: Option<AccountId>,
        solana: Option<AccountId>,
    }

    /// This object is responsible for verifying and proving trade execution done in another consensus network
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct TradeExecutionProof<BlockNumber> {
        pub target_network: Networks,
        pub target_network_blocknumber: BlockNumber,
        pub transaction_inclusion: TransactionInclusionProof,
        pub state_proof: StateProof,
        pub consensus_proof: ConsensusProofs,
    }

    /// Data to verify inclusion of the trade transaction
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct TransactionInclusionProof {
        tx_id: BoundedVec<u8, ConstU32<4_294_967_295>>,
        tx_proof: BoundedVec<BoundedVec<u8, ConstU32<4_294_967_295>>, ConstU32<4_294_967_295>>,
        key: BoundedVec<u8, ConstU32<4_294_967_295>>,
        tx_state_root: BoundedVec<u8, ConstU32<4_294_967_295>>,
    }

    /// Data to verify and read account balance after trade transaction
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct StateProof {
        pub state_root: BoundedVec<u8, ConstU32<4_294_967_295>>,
        pub state_proofs: BoundedVec<Vec<u8>, ConstU32<4_294_967_295>>,
        pub state_key: BoundedVec<u8, ConstU32<4_294_967_295>>,
    }

    /// Data to verify the canonical state of the target state machine
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct ConsensusProofs {
        pub consensus_root: Option<BoundedVec<u8, ConstU32<4_294_967_295>>>,
        pub consensus_proofs:
            Option<BoundedVec<BoundedVec<u8, ConstU32<4_294_967_295>>, ConstU32<4_294_967_295>>>,
        pub consensus_digest: Option<BoundedVec<u8, ConstU32<4_294_967_295>>>,
        pub consensus_digest_key: Option<BoundedVec<u8, ConstU32<4_294_967_295>>>,
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
        fn allocate_capital(network: Networks, trader_id: AccountIdFor<T>) -> DispatchResult;
    }

    impl<T: Config> CapitalAllocator<T> for () {
        fn allocate_capital(network: Networks, trader_id: AccountIdFor<T>) -> DispatchResult {
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
            network: Networks,
            proofs: TradeExecutionProof<BlockNumberFor<T>>,
            trade_action: TradeAction,
        ) -> TransactionValidity;

        // Verify trade transaction inclusion in the block of the target network ( Blockchain )
        fn verify_trade_tx_inclusion(network: Networks, proofs: TransactionInclusionProof) -> bool;

        // Verify state proofs and read the account balance
        fn verify_state_acount_balance(
            trader_id: AccountIdFor<T>,
            network: Networks,
            proofs: StateProof,
        ) -> u128;

        // Verify consensus commitment on N blockheight
        fn verify_consensus_state(network: Networks, proofs: ConsensusProofs) -> bool;
    }

    impl<T: Config> TradeExecutionVerifier<T> for () {
        fn verify_trade_execution(
            trader_id: AccountIdFor<T>,
            network: Networks,
            proofs: TradeExecutionProof<BlockNumberFor<T>>,
            trade_action: TradeAction,
        ) -> TransactionValidity {
            let is_consensus_valid = T::TradeExecutionVerifier::verify_consensus_state(
                network.clone(),
                proofs.consensus_proof,
            );

            let is_tx_valid = T::TradeExecutionVerifier::verify_trade_tx_inclusion(
                network.clone(),
                proofs.transaction_inclusion,
            );

            let state_account_balance = T::TradeExecutionVerifier::verify_state_acount_balance(
                trader_id,
                network,
                proofs.state_proof,
            );

            // Modify this to be dynamic in terms of priority,
            // All polkadot related verification should have lesser priorioty than non polkadot trade verification
            Ok(ValidTransaction {
                priority: u64::MAX,
                requires: vec![],
                provides: vec![],
                longevity: TransactionLongevity::MAX,
                propagate: true,
            })
        }

        fn verify_consensus_state(network: Networks, proofs: ConsensusProofs) -> bool {
            true
        }

        fn verify_state_acount_balance(
            trader_id: AccountIdFor<T>,
            network: Networks,
            proofs: StateProof,
        ) -> u128 {
            0
        }

        fn verify_trade_tx_inclusion(network: Networks, proofs: TransactionInclusionProof) -> bool {
            true
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

    //         let trading_roi: u128 /*This should asset id type */ = Decode::decode(&mut &encoded_balance[..])
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

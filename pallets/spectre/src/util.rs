#![cfg_attr(not(feature = "std"), no_std)]


use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use frame_support::DefaultNoBound;
use sp_std::vec::Vec;
use sp_std::vec;


use super::pallet::*;

pub use utils::*;

pub mod utils {

    use sp_arithmetic::Permill;

    use super::*;

    /// Tracking investor investments
    /// `deposited_capital`: Total capital deposited/ contributed to the pool
    /// `lp_ownership`: Total pool percentage ownerhip per ownership
    /// `accumulated profit`: Total points representing profits to be later claimed 
    /// `withdraw_period`: Total time that should elapse for investor to withdraw capital + profit
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct InvestorProfile<T: Config> {
        pub deposited_capital: T::AssetId, //BalanceOf<T>,this should be asset type not balances
        pub lp_ownership: Permill,
        pub block_number: BlockNumberFor<T>,
        pub accumulated_profit: u32,
        pub withdraw_period: BlockNumberFor<T>,
    }
    
    /// Tracking Trader activities
    /// `trading account`: The linked on chain trading account per trader sovereign account
    /// `bonded amount`: Amount placed into hold by the trader signifying conviction
    /// `funds allocated`: Total amount allocated to trader from pool
    /// `credits`: Metrics to measure trader performance
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct TraderProfile<T: Config>{
        pub trading_acount: Option<T::AccountId>,
        pub bonded_amount: TraderBond<T>,
        pub funds_allocated: u128,//BalanceOf<T>,
        pub credits: u8,
        pub trades_executed: u16,
    }

    /// Capital Pool management
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct InvestorLP<T: Config> {
        pub asset_name: BoundedVec<u8, ConstU32<20>>,
        pub total_capital: u128,//BalanceOf<T>,
        pub created_at: BlockNumberFor<T>,
        pub fee: u128//BalanceOf<T>,
    }

    /// Trader bond details and indicator if the bond should be staked for more rewards
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct TraderBond<T: Config> {
        pub amount: T::AssetId,
        pub stake: bool,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug,DefaultNoBound, MaxEncodedLen, TypeInfo)]
        pub struct TradingAccounts<AccountId>{
        substrate: Option<AccountId>,
        ethereum: Option<AccountId>,
        solana: Option<AccountId>
    }

    /// This object is responsible for verifying and proving trade execution done in another consensus network
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct TradeExecutionProof<BlockNumber> {
        pub consensus_root: Option<BoundedVec<u8,ConstU32<4_294_967_295>>>,
        pub consensus_proofs: Option<BoundedVec<BoundedVec<u8, ConstU32<4_294_967_295>>, ConstU32<4_294_967_295>>>,
        pub consensus_digest: Option<BoundedVec<u8, ConstU32<4_294_967_295>>>,
        pub consensus_digest_key: Option<BoundedVec<u8,ConstU32<4_294_967_295>>>,
        pub extrinsic_root: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub extrinsic_proofs: BoundedVec<Vec<u8>,ConstU32<4_294_967_295>>,
        pub extrinsic_data: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub extrinsic_key: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub state_root: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub state_proofs:BoundedVec<Vec<u8>,ConstU32<4_294_967_295>>,
        pub state_key: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub target_network_blocknumber: BlockNumber,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    pub enum Networks {
        Substrate,
        Ethereum,
        Solana,
        Sei
    }

    // Traits and implementations

    /// Responsible for allocating funds from different pools to trader on chain trading account
    pub trait CapitalAllocator<T: Config> {
        fn allocate_capital(network: Networks, trader_id:T::AccountId) -> DispatchResult;
    }

    impl<T: Config> CapitalAllocator<T> for () {
        fn allocate_capital(network: Networks, trader_id:T::AccountId) -> DispatchResult {
           Ok(()) 
        }
    }


    /// Responsible for verifying trade execution proofs
    pub trait TradeExecutionVerifier<T: Config> {
        fn verify_trade_execution(trader_id: T::AccountId,network: Networks, proofs: TradeExecutionProof<BlockNumberFor<T>>) -> TransactionValidity;
    }

    impl<T: Config> TradeExecutionVerifier<T> for () {
        fn verify_trade_execution(trader_id: T::AccountId,network: Networks, proofs: TradeExecutionProof<BlockNumberFor<T>>) -> TransactionValidity {
            // Modify this to be dynamic in terms of priority, 
                    // All polkadot related verification should have lesser priorioty than non polkadot trade verification
                    Ok(ValidTransaction{
                        priority: u64::MAX,
                        requires: vec![],
                        provides: vec![],
                        longevity: TransactionLongevity::MAX,
                        propagate: true
                    })
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

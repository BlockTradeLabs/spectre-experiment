#![cfg_attr(not(feature = "std"), no_std)]


use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use frame_support::DefaultNoBound;
use sp_std::vec::Vec;

use super::pallet::*;

pub use utils::*;

pub mod utils {

    use super::*;

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct InvestorProfile<T: Config> {
        pub deposited_capital: u128, //BalanceOf<T>,this should be asset type not balances
        pub lp_ownership: u128,
        pub block_number: BlockNumberFor<T>,
        pub accumulated_profit: u128,//BalanceOf<T>,
        pub withdraw_period: BlockNumberFor<T>,
    }
    
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct TraderProfile<T: Config>{
        pub trading_acount: Option<T::AccountId>,
        pub bonded_amount: TraderBond,
        pub funds_allocated: u128,//BalanceOf<T>,
        pub credits: u8,
        pub trades_executed: u16,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct InvestorLP<T: Config> {
        pub asset_name: BoundedVec<u8, ConstU32<20>>,
        pub total_capital: u128,//BalanceOf<T>,
        pub created_at: BlockNumberFor<T>,
        pub fee: u128//BalanceOf<T>,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	//#[scale_info(skip_type_params(T))]
    pub struct TraderBond {
        pub amount: u128,//BalanceOf<T>,
        pub stake: bool,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug,DefaultNoBound, MaxEncodedLen, TypeInfo)]
        pub struct TradingAccounts<AccountId>{
        substrate: Option<AccountId>,
        ethereum: Option<AccountId>,
        solana: Option<AccountId>
    }

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

    // Traits
    pub trait CapitalAllocator<T: Config> {
        fn allocate_capital(network: Networks, trader_id:T::AccountId) -> DispatchResult;
    }

    impl<T: Config> CapitalAllocator<T> for () {
        fn allocate_capital(network: Networks, trader_id:T::AccountId) -> DispatchResult {
           Ok(()) 
        }
    }
}

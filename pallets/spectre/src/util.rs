#![cfg_attr(not(feature = "std"), no_std)]


use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use frame_support::DefaultNoBound;

use super::pallet::*;

pub use utils::*;

pub mod utils {

    use super::*;

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct InvestorProfile<T: Config> {
        pub deposited_capital: BalanceOf<T>,
        pub lp_ownership: u128,
        pub block_number: BlockNumberFor<T>,
        pub accumulated_profit: BalanceOf<T>,
        pub withdraw_period: BlockNumberFor<T>,
    }
    
    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct TraderProfile<T: Config>{
        pub trading_acount: Option<T::AccountId>,
        pub bonded_amount: TraderBond<T>,
        pub funds_allocated: BalanceOf<T>,
        pub credits: u8,
        pub trades_executed: u16,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct InvestorLP<T: Config> {
        pub asset_name: BoundedVec<u8, ConstU32<20>>,
        pub total_capital: BalanceOf<T>,
        pub created_at: BlockNumberFor<T>,
        pub fee: BalanceOf<T>,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct TraderBond<T:Config> {
        pub amount: BalanceOf<T>,
        pub stake: bool,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug,DefaultNoBound, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct TradingAccounts<T: Config>{
        substrate: Option<T::AccountId>,
        ethereum: Option<T::AccountId>,
        solana: Option<T::AccountId>
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug,DefaultNoBound, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
    pub struct TradeExecutionProof<T: Config> {
        pub consensus_root: Option<BoundedVec<u8,ConstU32<4_294_967_295>>>,
        pub consensus_proofs: Option<BoundedVec<BoundedVec<u8, ConstU32<4_294_967_295>>, ConstU32<4_294_967_295>>>,
        pub consensus_digest: Option<BoundedVec<u8, ConstU32<4_294_967_295>>>,
        pub consensus_digest_key: Option<BoundedVec<u8,ConstU32<4_294_967_295>>>,
        pub extrinsic_root: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub extrinsic_proofs: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub extrinsic_data: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub extrinsic_key: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub state_root: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub state_proofs:BoundedVec<BoundedVec<u8, ConstU32<4_294_967_295>>, ConstU32<4_294_967_295>>,
        pub state_key: BoundedVec<u8,ConstU32<4_294_967_295>>,
        pub target_network_blocknumber: BlockNumberFor<T>,
    }

    #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    pub enum Networks {
        Substrate,
        Ethereum,
        Solana,
        Sei
    }
}

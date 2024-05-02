use std::sync::Once;
use emulated_integration_tests_common::{get_account_id_from_seed, build_genesis_storage,};
use xcm_emulator::*;
use polkadot_core_primitives::AccountPublic;
use sp_core::{ecdsa,H160,Public as EcdsaPublic, ecdsa::Pair as PairType};
use sp_core::crypto::Ss58AddressFormatRegistry;
use sp_runtime::MultiSigner;
use sp_runtime::traits::IdentifyAccount;
use sp_core::crypto::Ss58Codec;
use sp_runtime::BuildStorage;
use staging_xcm_executor::traits::ConvertLocation;
use frame_support::traits::UnfilteredDispatchable;
use frame_support::pallet_prelude::*;
use rococo_emulated_chain::{genesis};
use asset_hub_rococo_emulated_chain::AssetHubRococo;
use cumulus_primitives_core::relay_chain::runtime_api::runtime_decl_for_parachain_host::ParachainHostV10;

const SAFE_XCM_VERSION: u32 =  crate::XCM_VERSION;
pub const ORCHESTRATOR: ParaId = ParaId::new(1000);


#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RococoId(u32);

// fn calculate_sovereign_account<Pair>(
// 	para_id: u32,
// ) -> Result<String, Box<dyn std::error::Error>>
// 	where
// 		Pair: sp_core::Pair,
// 		Pair::Public: Into<MultiSigner>,
// {
// 	// Scale encoded para_id
// 	let id = RococoId(para_id);
// 	let encoded_id = hex::encode(id.encode());

// 	// Prefix para or sibl
// 	let prefix = hex::encode("para");

// 	// Join both strings and the 0x at the beginning
// 	let encoded_key = "0x".to_owned() + &prefix + &encoded_id;

// 	// Fill the rest with 0s
// 	let public_str = format!("{:0<width$}", encoded_key, width = 64 + 2);

// 	// Convert hex public key to ss58 address
// 	let public = array_bytes::hex2bytes(&public_str).expect("Failed to convert hex to bytes");
// 	let public_key = Pair::Public::try_from(&public)
// 		.map_err(|_| "Failed to construct public key from given hex")?;

// 	Ok(public_key.to_ss58check_with_version(Ss58AddressFormatRegistry::SubstrateAccount.into()))
// }

pub mod accounts {
	use sp_core::ecdsa;
	use super::*;
	pub const ALICE: &str = "Alice";
	pub const BOB: &str = "Bob";
	pub const CHARLIE: &str = "Charlie";
	pub const DAVE: &str = "Dave";
	pub const EVE: &str = "Eve";


	pub fn init_balances() -> Vec<AccountId> {
		vec![
			get_account_id_from_seed::<ecdsa::Public>(ALICE),
			get_account_id_from_seed::<ecdsa::Public>(BOB),
			get_account_id_from_seed::<ecdsa::Public>(CHARLIE),
			get_account_id_from_seed::<ecdsa::Public>(DAVE),
			get_account_id_from_seed::<ecdsa::Public>(EVE),

		]
	}


	pub fn sudo_key() -> AccountId {
		get_account_id_from_seed::<ecdsa::Public>(ALICE)
	}
}



pub use spectre_finance_container::*;

pub mod spectre_finance_container {
	use super::*;
    use sp_core::crypto::Ss58Codec;
	use crate::{EXISTENTIAL_DEPOSIT,Balance,AssetId};
	use crate::integration_test::accounts::{ALICE, sudo_key};


	pub const PARA_ID: u32 = 2000;
	pub const ED: Balance = EXISTENTIAL_DEPOSIT;
	pub fn genesis() -> Storage {

		// Calculate parachain Soverign account id
		//let sovererign_acount = calculate_sovereign_account::<PairType>(PARA_ID.into()).unwrap();
		//let para_account = sp_runtime::AccountId32::from_ss58check(&sovererign_acount).unwrap();
        let alice:[u8;32] = get_account_id_from_seed::<ecdsa::Public>(ALICE).into();
    
		// ---******* GENESIS CONFIG ********---//


		let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

        let genesis_config = crate::RuntimeGenesisConfig {
            system: Default::default(),
            balances: crate::BalancesConfig {
                balances: vec![
					(alice.clone().into(),1_000_000)
				]
            },
            parachain_info: crate::ParachainInfoConfig {
                parachain_id: PARA_ID.into(),
                ..Default::default()
            },
            parachain_system: Default::default(),
            // EVM compatibility
            // We should change this to something different than Moonbeam
            // For now moonwall is very tailored for moonbeam so we need it for tests
            evm_chain_id: crate::EVMChainIdConfig {
                chain_id: 1281u32 as u64,
                ..Default::default()
            },
            evm: crate::EVMConfig {
                // We need _some_ code inserted at the precompile address so that
                // the evm will actually call the address.
                ..Default::default()
            },
            ethereum: Default::default(),
            base_fee: Default::default(),
            transaction_payment: Default::default(),
            sudo: crate::SudoConfig {
                key: Some(alice.into()),
            },
            authorities_noting: crate::AuthoritiesNotingConfig {
                orchestrator_para_id: ORCHESTRATOR,
                ..Default::default()
            },
            migrations: crate::MigrationsConfig {
                ..Default::default()
            },
            maintenance_mode: crate::MaintenanceModeConfig {
                start_in_maintenance_mode: false,
                ..Default::default()
            },
            // This should initialize it to whatever we have set in the pallet
            polkadot_xcm: crate::PolkadotXcmConfig::default(),
            tx_pause: Default::default(),
            asset_registry: crate::AssetRegistryConfig {
                assets: vec![
                    (1, b"sfDOT".to_vec()),
                    (2, b"sfUSDT".to_vec()),
                    (3, b"sfUSDC".to_vec()),
                ],
                last_asset_id: 3,
            },
            assets: crate::AssetsConfig { balances: vec![] },
            spectre: crate::SpectreConfig {
                relayer: Some(alice.into()),
                initial_capital: 0,
                supported_assets: vec![],
                fee: 10, // percentage
            }

        };

        build_genesis_storage(&genesis_config, crate::WASM_BINARY.unwrap())
    }
}



// Relay Network Implementation

decl_test_relay_chains! {
	#[api_version(5)]
	pub struct Rococo {
		genesis = genesis(),
		on_init = (),
		runtime = rococo_runtime,
		core = {
			SovereignAccountOf: rococo_runtime::xcm_config::LocationConverter,
		},
		pallets = {
			XcmPallet: rococo_runtime::XcmPallet,
			Balances: rococo_runtime::Balances,
			Hrmp: rococo_runtime::Hrmp,
		}
	}
}

decl_test_parachains! {
	pub struct SpectreFinanceContainer {
		genesis = genesis::genesis(),
		on_init = {
			();
		},
		runtime = crate,
		core = {
			XcmpMessageHandler: crate::XcmpQueue,
			LocationToAccountId: crate::xcm_config::LocationToAccountId,
			ParachainInfo: crate::ParachainInfo,
			MessageOrigin: crate::AggregateMessageOrigin,
		},
		pallets = {
            Scehduler: crate::Scheduler,
			PolkadotXcm: crate::PolkadotXcm,
			Assets: crate::Assets,
			Xtokens: crate::Xtokens,
			AssetRegistry: crate::AssetRegistry,
			Balances: crate::Balances,
            Spectre: crate::Spectre,
		}
	}
}



decl_test_networks!(
	// Rococo
	pub struct RococoNet {
		relay_chain = Rococo,
		parachains = vec![
			AssetHubRococo,
			SpectreFinanceContainer,
		],
		bridge = ()
	}
);


#[cfg(test)]
mod tests {
	use super::*;

	
	#[test]
	fn trader_registration_works(){

	}

	#[test]
    fn investor_registration_works(){

	}

	#[test]
	fn capital_allocation_works_dot(){

	}

    #[test]
    fn capital_allocation_works_stablecoin(){}

    #[test]
    fn trade_execution_verification_works(){}

    #[test]
    fn withdraw_works(){}
  
}
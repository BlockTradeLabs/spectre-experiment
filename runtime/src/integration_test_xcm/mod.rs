use {
    asset_hub_rococo_emulated_chain::{AssetHubRococo, AssetHubRococoParaPallet},
    cumulus_primitives_core::relay_chain::runtime_api::runtime_decl_for_parachain_host::ParachainHostV10,
    emulated_integration_tests_common::{
        accounts::{ALICE, BOB},
        build_genesis_storage, get_account_id_from_seed, test_parachain_is_trusted_teleporter,
        xcm_emulator::{
            assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
            RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
        },
        xcm_helpers::{xcm_transact_paid_execution, xcm_transact_unpaid_execution},
        PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, XCM_V3,
    },
    frame_support::{
        pallet_prelude::{DispatchResult, *},
        traits::UnfilteredDispatchable,
    },
    polkadot_core_primitives::AccountPublic,
    rococo_emulated_chain::{Rococo, RococoRelayPallet},
    sp_core::{
        crypto::{Ss58AddressFormatRegistry, Ss58Codec},
        ecdsa,
        ecdsa::Pair as PairType,
        Public as EcdsaPublic, H160,
    },
    sp_runtime::{traits::IdentifyAccount, BuildStorage, MultiSigner},
    staging_xcm_executor::traits::ConvertLocation,
    std::sync::Once,
    xcm_emulator::*,
};

const SAFE_XCM_VERSION: u32 = crate::XCM_VERSION;
pub const ORCHESTRATOR: ParaId = ParaId::new(1000);

#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RococoId(u32);

pub mod accounts {
    use {super::*, sp_core::ecdsa};
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
    use {
        super::*,
        crate::{
            integration_test_xcm::accounts::{sudo_key, ALICE},
            AssetId, Balance, EXISTENTIAL_DEPOSIT,
        },
        orml_traits::asset_registry::AssetMetadata,
        sp_core::crypto::Ss58Codec,
        staging_xcm::{latest::Junction, VersionedMultiLocation},
    };

    pub const PARA_ID: u32 = 2000;
    pub const ED: Balance = EXISTENTIAL_DEPOSIT;
    pub fn genesis() -> Storage {
        // Calculate parachain Soverign account id
        //let sovererign_acount = calculate_sovereign_account::<PairType>(PARA_ID.into()).unwrap();
        //let para_account = sp_runtime::AccountId32::from_ss58check(&sovererign_acount).unwrap();
        let alice: [u8; 32] = get_account_id_from_seed::<ecdsa::Public>(ALICE).into();

        // ---******* GENESIS CONFIG ********---//

        // asset metadata
        let sf_dot = AssetMetadata::<Balance, Vec<u8>, ConstU32<10>> {
            decimals: 12,
            name: BoundedVec::truncate_from("sfDOT".as_bytes().to_vec()),
            symbol: BoundedVec::truncate_from("sfDOT".as_bytes().to_vec()),
            existential_deposit: 0,
            location: Some(VersionedMultiLocation::V3(MultiLocation::new(
                0,
                Junction::GeneralIndex(1),
            ))),
            additional: b"spectre finance derived token".to_vec(),
        };
        let sf_usdt = AssetMetadata::<Balance, Vec<u8>, ConstU32<10>> {
            decimals: 12,
            name: BoundedVec::truncate_from("sfUSDT".as_bytes().to_vec()),
            symbol: BoundedVec::truncate_from("sfUSDT".as_bytes().to_vec()),
            existential_deposit: 0,
            location: Some(VersionedMultiLocation::V3(MultiLocation::new(
                0,
                Junction::GeneralIndex(2),
            ))),
            additional: b"spectre finance derived token".to_vec(),
        };
        let sf_usdc = AssetMetadata::<Balance, Vec<u8>, ConstU32<10>> {
            decimals: 12,
            name: BoundedVec::truncate_from("sfUSDC".as_bytes().to_vec()),
            symbol: BoundedVec::truncate_from("sUSDC".as_bytes().to_vec()),
            existential_deposit: 0,
            location: Some(VersionedMultiLocation::V3(MultiLocation::new(
                0,
                Junction::GeneralIndex(3),
            ))),
            additional: b"spectre finance derived token".to_vec(),
        };

        let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

        let genesis_config = crate::RuntimeGenesisConfig {
            system: Default::default(),
            balances: crate::BalancesConfig {
                balances: vec![(alice.clone().into(), 1_000_000)],
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
                    (1, sf_dot.encode()),
                    (2, sf_usdt.encode()),
                    (3, sf_usdc.encode()),
                ],
                last_asset_id: 3,
            },
            assets: crate::AssetsConfig { balances: vec![] },
            spectre: crate::SpectreConfig {
                relayer: Some(alice.into()),
                initial_capital: 0,
                supported_assets: vec![],
                fee: 10, // percentage
            },
        };

        build_genesis_storage(&genesis_config, crate::WASM_BINARY.unwrap())
    }
}

decl_test_parachains! {
    pub struct SpectreFinanceContainer {
        genesis = genesis(),
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

// ================================ Helper types and functions ========================== //
pub type RelayToSystemParaTest = Test<Rococo<RococoNet>, AssetHubRococo<RococoNet>>;
pub type RelayToParaTest = Test<Rococo<RococoNet>, SpectreFinanceContainer<RococoNet>>;
pub type SystemParaToRelayTest = Test<AssetHubRococo<RococoNet>, Rococo<RococoNet>>;
pub type SystemParaToParaTest = Test<AssetHubRococo<RococoNet>, SpectreFinanceContainer<RococoNet>>;
pub type ParaToSystemParaTest = Test<SpectreFinanceContainer<RococoNet>, AssetHubRococo<RococoNet>>;

decl_test_sender_receiver_accounts_parameter_types! {
    RococoRelay { sender: ALICE, receiver: BOB },
    AssetHubRococoPara { sender: ALICE, receiver: BOB },
    SpectreFinanceContainerPara { sender: ALICE, receiver: BOB }
}

// fn relay_to_para_sender_assertions(t: RelayToParaTest) {
// 	type RuntimeEvent = <Rococo::<RococoNet> as Chain>::RuntimeEvent;
// 	Rococo::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(864_610_000, 8_799)));
// 	assert_expected_events!(
// 		Rococo<RococoNet>,
// 		vec![
// 			// Amount to reserve transfer is transferred to Parachain's Sovereign account
// 			RuntimeEvent::Balances(
// 				pallet_balances::Event::Transfer { from, to, amount }
// 			) => {
// 				from: *from == t.sender.account_id,
// 				to: *to == Rococo::sovereign_account_id_of(
// 					t.args.dest
// 				),
// 				amount: *amount == t.args.amount,
// 			},
// 		]
// 	);
// }
//
// fn system_para_to_para_sender_assertions(t: SystemParaToParaTest) {
// 	type RuntimeEvent = <AssetHubRococo::<RococoNet> as Chain>::RuntimeEvent;
// 	AssetHubRococo::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(
// 		864_610_000,
// 		8_799,
// 	)));
// 	assert_expected_events!(
// 		AssetHubRococo,
// 		vec![
// 			// Amount to reserve transfer is transferred to Parachain's Sovereign account
// 			RuntimeEvent::Balances(
// 				pallet_balances::Event::Transfer { from, to, amount }
// 			) => {
// 				from: *from == t.sender.account_id,
// 				to: *to == AssetHubRococo::sovereign_account_id_of(
// 					t.args.dest
// 				),
// 				amount: *amount == t.args.amount,
// 			},
// 		]
// 	);
// }
//
// fn para_receiver_assertions<Test>(_: Test) {
// 	type RuntimeEvent = <SpectreFinanceContainer::<RococoNet> as Chain>::RuntimeEvent;
// 	assert_expected_events!(
// 		SpectreFinanceContainer,
// 		vec![
// 			RuntimeEvent::Balances(pallet_balances::Event::Deposit { .. }) => {},
// 			RuntimeEvent::MessageQueue(
// 				pallet_message_queue::Event::Processed { success: true, .. }
// 			) => {},
// 		]
// 	);
// }

fn relay_to_para_reserve_transfer_assets(t: RelayToParaTest) -> DispatchResult {
    <Rococo<RococoNet> as RococoRelayPallet>::XcmPallet::limited_reserve_transfer_assets(
        t.signed_origin,
        bx!(t.args.dest.into()),
        bx!(t.args.beneficiary.into()),
        bx!(t.args.assets.into()),
        t.args.fee_asset_item,
        t.args.weight_limit,
    )
}

fn system_para_to_para_reserve_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
    <AssetHubRococo::<RococoNet> as AssetHubRococoParaPallet>::PolkadotXcm::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

#[cfg(test)]
mod tests {
    use {super::*, fp_account::AccountId20, hex_literal::hex};

    #[test]
    fn trader_registration_works() {
        // check alice balance in relay chain
        let destination =
            Rococo::<RococoNet>::child_location_of(SpectreFinanceContainer::<RococoNet>::para_id());
        let beneficiary_id = SpectreFinanceContainerParaReceiver::get();
        let amount_to_send = 100000;

        let alice_sender_para = AccountId20::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"));
        let bob_receiver = AccountId20::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"));

        let test_args = TestContext {
            sender: RococoRelaySender::get(),
            receiver: bob_receiver,
            args: TestArgs::new_relay(destination, beneficiary_id, amount_to_send),
        };

        let mut relay_to_para_test = RelayToParaTest::new(test_args);

        let sender_balance_before = relay_to_para_test.sender.balance;
        let receiver_balance_before = relay_to_para_test.receiver.balance;

        // Reserve transfer assets
        relay_to_para_test
            .set_dispatchable::<Rococo<RococoNet>>(relay_to_para_reserve_transfer_assets);
        relay_to_para_test.assert();
    }

    #[test]
    fn investor_registration_works() {}

    #[test]
    fn capital_allocation_works_dot() {}

    #[test]
    fn capital_allocation_works_stablecoin() {}
}

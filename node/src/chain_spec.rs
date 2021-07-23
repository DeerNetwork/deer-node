//! Substrate chain configurations.

use sc_chain_spec::{ChainSpecExtension, Properties};
use sp_core::{Pair, Public, crypto::UncheckedInto, sr25519};
use serde::{Serialize, Deserialize};
use node_runtime::{
	GenesisConfig, AuthorityDiscoveryConfig, BabeConfig, BalancesConfig, CouncilConfig,
	DemocracyConfig, GrandpaConfig, ImOnlineConfig, SessionConfig, SessionKeys, StakerStatus,
	StakingConfig, ElectionsConfig, IndicesConfig, SudoConfig, SystemConfig, Block,
	TechnicalCommitteeConfig, wasm_binary_unwrap, MAX_NOMINATIONS,
};
use node_runtime::constants::currency::*;
use sc_service::ChainType;
use hex_literal::hex;
use grandpa_primitives::{AuthorityId as GrandpaId};
use sp_consensus_babe::{AuthorityId as BabeId};
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_runtime::{Perbill, traits::{Verify, IdentifyAccount}};

pub use node_primitives::{AccountId, Balance, Signature};

type AccountPublic = <Signature as Verify>::Signer;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<
	GenesisConfig,
	Extensions,
>;

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys { grandpa, babe, im_online, authority_discovery }
}

fn staging_testnet_config_genesis() -> GenesisConfig {
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	// and
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
		// 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
		hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
		// 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
		hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
		// 5Fb9ayurnxnaXj56CjmyQLBiadfRCqUbL2VWNbbe1nZU6wiC
		hex!["9becad03e6dcac03cee07edebca5475314861492cdfc96a2144a67bbe9699332"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
	),(
		// 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
		hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
		// 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
		hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
		// 5EockCXN6YkiNCDjpqqnbcqd4ad35nU4RmA1ikM4YeRN4WcE
		hex!["7932cff431e748892fa48e10c63c17d30f80ca42e4de3921e641249cd7fa3c2f"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
	),(
		// 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
		hex!["547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"].into(),
		// 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
		hex!["9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"].into(),
		// 5E1jLYfLdUQKrFrtqoKgFrRvxM3oQPMbf6DfcsrugZZ5Bn8d
		hex!["5633b70b80a6c8bb16270f82cca6d56b27ed7b76c8fd5af2986a25a4788ce440"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
	),(
		// 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
		hex!["f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"].into(),
		// 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
		hex!["66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"].into(),
		// 5DMa31Hd5u1dwoRKgC4uvqyrdK45RHv3CpwvpUC1EzuwDit4
		hex!["3919132b851ef0fd2dae42a7e734fe547af5a6b809006100f48944d7fae8e8ef"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
	)];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// 5Ff3iXP75ruzroPWRP2FYBHWnmGGBSb63857BgnzCoXNxfPo
		"9ee5e5bdc0ec239eb164f865ecc345ce4c88e76ee002e0f7e318097347471809"
	].into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	testnet_genesis(initial_authorities, vec![], root_key, Some(endowed_accounts))
}

/// Staging testnet config.
pub fn staging_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Staging Testnet",
		"staging_testnet",
		ChainType::Live,
		staging_testnet_config_genesis,
		vec![],
		None,
		None,
		None,
		Default::default(),
	)
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(seed: &str) -> (
	AccountId,
	AccountId,
	GrandpaId,
	BabeId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	initial_nominators: Vec<AccountId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		]
	});
	// endow all authorities and nominators.
	initial_authorities.iter().map(|x| &x.0).chain(initial_nominators.iter()).for_each(|x| {
		if !endowed_accounts.contains(&x) {
			endowed_accounts.push(x.clone())
		}
	});

	// stakers: all validators and nominators.
	let mut rng = rand::thread_rng();
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|x| {
			use rand::{seq::SliceRandom, Rng};
			let limit = (MAX_NOMINATIONS as usize).min(initial_authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = initial_authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.into_iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	const STASH: Balance = ENDOWMENT / 1000;

	GenesisConfig {
		system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|x| (x, ENDOWMENT))
				.collect()
		},
		indices: IndicesConfig {
			indices: vec![],
		},
		session: SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), session_keys(
					x.2.clone(),
					x.3.clone(),
					x.4.clone(),
					x.5.clone(),
				))
			}).collect::<Vec<_>>(),
		},
		staking: StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			.. Default::default()
		},
		democracy: DemocracyConfig::default(),
		elections: ElectionsConfig {
			members: endowed_accounts.iter()
						.take((num_endowed_accounts + 1) / 2)
						.cloned()
						.map(|member| (member, STASH))
						.collect(),
		},
		council: CouncilConfig::default(),
		technical_committee: TechnicalCommitteeConfig {
			members: endowed_accounts.iter()
						.take((num_endowed_accounts + 1) / 2)
						.cloned()
						.collect(),
			phantom: Default::default(),
		},
		sudo: SudoConfig {
			key: root_key,
		},
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(node_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		im_online: ImOnlineConfig {
			keys: vec![],
		},
		authority_discovery: AuthorityDiscoveryConfig {
			keys: vec![],
		},
		grandpa: GrandpaConfig {
			authorities: vec![],
		},
		technical_membership: Default::default(),
		treasury: Default::default(),
		vesting: Default::default(),
		file_storage: Default::default(),
		transaction_storage: Default::default(),
	}
}

fn development_config_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![
			authority_keys_from_seed("Alice"),
		],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		development_config_genesis,
		vec![],
		None,
		None,
		None,
		Default::default(),
	)
}

fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
		],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		local_testnet_genesis,
		vec![],
		None,
		None,
		None,
		Default::default(),
	)
}

fn nft360_testnet_config_genesis() -> GenesisConfig {
	// ./scripts/prepare-test-net.sh 4
	let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![
		(
			//5CoeCei9ULeUXKKRY4MDq3Bpp2JH5JVdRHfU4B2UZsKaGtNm
			hex!["20bf3f380a5d4888e2f7c467bb03abf0efd8780c4ea0cdf8b1ec2e70d24e2249"].into(),
			//5HdL6KCaMwxbAR3j7wY25DW3HcrbxQLXpLqGPCupec2Fne2L
			hex!["f60effbef7654a6590fca2b94edc9fcd80434aa08ad3ecf8c9514851cbe5057a"].into(),
			//5CJkZKD4rRqEhHphBWvVH8kPipjccrWk7F1a8xbZgvsnt2aG
			hex!["0ab686ffd031340ca2fb1101d8f87af4f15046df6363d0e9ce9084ab62cce95d"].unchecked_into(),
			//5EByXQB4WQ98sJmx4aUeTsqmT9j526SZEoj1UZJwB2c63UWE
			hex!["5e03ed5d6cca04b81f110c9704e83d0d4b291a0e04493007d3114a45949dd935"].unchecked_into(),
			//5H6L81P5gQZmTaJby6CRhMK1gFCWUgQAKQv7b8yEpH7bMMs5
			hex!["de6a7d5e562776ea64b3d8d67b326597512d787bb9a5477b56cab205a1bdcb62"].unchecked_into(),
			//5HN22KVWJHrRZRwxz8mDrydKRFYnRMm477q5voKE4J67655B
			hex!["ea617e1fe815de26e97ab5cb74f868a1f427a08abedb9ebc1d5c50f0473afe50"].unchecked_into(),
		),
		(
			//5DAmdPpeV7SPB7eKb8UbBBrTMfKGrGrVCtZCu7rsPjzksHHt
			hex!["30dc668ff7fb462ed059f5de128f46654f71858b38e22f0cf64a53158783ca1d"].into(),
			//5EqKaozw6DP3QrCnmjgrr1S5kWAvJqrCZyEtMq3M6bGszGKQ
			hex!["7a7f87f82846340224af31c6b5305bdd332ce93eae157c2b1aaa9b58fc1ef374"].into(),
			//5HRfmmXSciAfCtQCPRZ2Xz94D3F2nTuC6WEwpkD96w3dj5Ti
			hex!["ed2a4dc256fb49b8956790a3bc3b50188e6903a7344b57882c4c32d34d29721a"].unchecked_into(),
			//5CQ5vYWJcjXt4PiL8uh8Evyzx8imhyg8Wn5ADiRDC599YEre
			hex!["0ec7f23a5fa3fe572ffb34473c1e28b1b2768ed1e76675d1aa292c53a7682002"].unchecked_into(),
			//5Fmr5S6FM7rFcfYVXjXVuLABEV9wN67i9bs3RWDXdbkvaWaX
			hex!["a4156ee5daea942eebdd55339483a59db2112eb9ca2209781170afc29942f745"].unchecked_into(),
			//5DAkFCvxMLK6mAByFE6RotATqprCH4sGmPyfRT9hfLNL3JvA
			hex!["30d7bf1ec2186c7922958cdc634e51aafbaaf19e98dd2f48443eb8eaf7c2ef56"].unchecked_into(),
		),
		(
			//5GVtzvvzFY81WLyMajvLSVS5BebzTRBkdhJReocH75LCefgL
			hex!["c42793e6c4dd445202c478ea8757fcb772c54c7404f366090ecf5b9461123e38"].into(),
			//5HmAsCXEv7qf1Ct7Khwt8qvbq6poiHKro1DQiqtnAsyeapqs
			hex!["fc09e7830fb8509d4e9a9791c18dbf0dd0d2adacd340f6d5213bf55ea4bc2d2b"].into(),
			//5GiMEz7HFB3kL6NqFe6DZUy8KVbvfpFZY2kfy3N85YRpxWe5
			hex!["cda6dcb47ea1d0107d49314c7703784a0353a20e9dc52d9545dceb161dc4a229"].unchecked_into(),
			//5CXD8tyrkhxsHqkYNQWXAeSzQzhtsz2XfQoshBr9f14fxJZv
			hex!["1436f1f205b2537f090f30a45f7f5df08001d212d68b5a1a99f078d5b7c8ca22"].unchecked_into(),
			//5Cqfe3y575fEGdDtRYtgwxtFyuGdaHepW8nFU8a5tRaCwPyM
			hex!["224a940cee0e91d36c38edc0e901c39f0de4c4bfe9785039db9e37c640c4832f"].unchecked_into(),
			//5DF4VLHLBgJ4Fn8bv1F9rVQCGh16CTAvDm82Gm7Yc7Yp5HV4
			hex!["34222438e9a2dd5417e9c0952365bf136969eac2759775d8505a1bcb41b81332"].unchecked_into(),
		),
		(
			//5CCzfrdLEtFGG3F4TU93R5Qjuyki9JKCHoGAFFPcNZ4D4UQc
			hex!["065290133d4c90bbdb535ee0c407ce2d89e63d7e2938b183801af21cf386b842"].into(),
			//5F4LDWLGdSwGw6UUM7XAnvJfjGjCsnQFcaXvnumMvRscTqcb
			hex!["846bd9e6a583f31c7d7a99d350fa78e9bde049517d311efba426e73f1c945f20"].into(),
			//5Fi7xnxhzKiQLiZngV9hqwEQya9fhyc5aQt8aUrs73GWRMpp
			hex!["a13ded4ee76fbf579a2f8a882fb96f9ddce9b8c5a52f4a8bbfc02def6e96efb2"].unchecked_into(),
			//5F92uUnZpyjqugxfSUCVthfxyK9jwE9pTxZKJocc7n12MJx3
			hex!["8801ce09a54c7c65db57983f5b04431043e4999a936a9cd955a755d57ba5315a"].unchecked_into(),
			//5CyV1HXSGYfWiLB1e9mZ5pZHX5KurqrWtRwxMvBe4i6ifTxh
			hex!["2840bea1cdcfdf1f4ef192b487e23a3bef7babae1da8e494a73f55b974649757"].unchecked_into(),
			//5HRLdeWuTwy7uya6V46h2jgKf4WBZ1S8nsudbCngpnKP6jfx
			hex!["ece9df6a2db6cc1d563fa84a6e7ddb5bbfd7751897d80f212dc3e12c72c0a10d"].unchecked_into(),
		),
	];
	// generated with secret: subkey inspect --scheme sr25519 "$SECRET"/n360t
	let root_key: AccountId = hex!["624db4345f31b2432be2cc2cf6305ac0a1bd119b142ba1da7dffe27092529e4b"].into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	testnet_genesis(
		initial_authorities,
		vec![],
		root_key,
		Some(endowed_accounts),
	)
}

pub fn nft360_testnet_local_config() -> ChainSpec {
	let boot_nodes = vec![];
	let protocol_id: &str = "n360t1";
	let properties = {
		let mut p = Properties::new();
		p.insert("tokenSymbol".into(), "N360".into());
		p.insert("tokenDecimals".into(), 12.into());
		p.insert("ss58Format".into(), 333.into());
		p
	};

	ChainSpec::from_genesis(
		"NFT360 PoC-1",
		"nft360_poc_1",
		ChainType::Local,
		nft360_testnet_config_genesis,
		boot_nodes,
		None,
		Some(protocol_id),
		Some(properties),
		Default::default(),
	)
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use sp_runtime::BuildStorage;

	fn local_testnet_genesis_instant_single() -> GenesisConfig {
		testnet_genesis(
			vec![
				authority_keys_from_seed("Alice"),
			],
			vec![],
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			None,
		)
	}

	/// Local testnet config (single validator - Alice)
	pub fn integration_test_config_with_single_authority() -> ChainSpec {
		ChainSpec::from_genesis(
			"Integration Test",
			"test",
			ChainType::Development,
			local_testnet_genesis_instant_single,
			vec![],
			None,
			None,
			None,
			Default::default(),
		)
	}

	/// Local testnet config (multivalidator Alice + Bob)
	pub fn integration_test_config_with_two_authorities() -> ChainSpec {
		ChainSpec::from_genesis(
			"Integration Test",
			"test",
			ChainType::Development,
			local_testnet_genesis,
			vec![],
			None,
			None,
			None,
			Default::default(),
		)
	}

	#[test]
	fn test_create_development_chain_spec() {
		development_config().build_storage().unwrap();
	}

	#[test]
	fn test_create_local_testnet_chain_spec() {
		local_testnet_config().build_storage().unwrap();
	}

	#[test]
	fn test_staging_test_net_chain_spec() {
		staging_testnet_config().build_storage().unwrap();
	}
}

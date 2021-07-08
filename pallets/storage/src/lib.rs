//! # Storage Online Module

#![cfg_attr(not(feature = "std"), no_std)]


// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

mod constants;
pub use constants::*;

// pub mod weights;


use sp_std::{
	prelude::*,
	collections::btree_map::BTreeMap,
};
use sp_runtime::{Perbill, RuntimeDebug, SaturatedConversion, traits::{Zero, One, StaticLookup, Saturating, AccountIdConversion}};
use codec::{Encode, Decode};
use frame_support::{
	traits::{Currency, ReservableCurrency, ExistenceRequirement, UnixTime, Get},
};
use frame_system::{Config as SystemConfig, pallet_prelude::BlockNumberFor};
use p256::ecdsa::{VerifyingKey, signature::{Verifier, Signature}};

pub type RootId = Vec<u8>;
pub type EnclaveId = Vec<u8>;
pub type PubKey = Vec<u8>;
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;
pub type RoundIndex = u32;

// pub use weights::WeightInfo;
pub use pallet::*;

// syntactic sugar for logging.
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: crate::LOG_TARGET,
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

pub trait RoundPayout<Balance> {
	fn round_payout(total_size: u128) -> Balance;
}

impl<Balance: Default> RoundPayout<Balance> for () {
	fn round_payout(_total_size: u128) -> Balance {
		Default::default()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct NodeInfo<BlockNumber> {
	pub last_reported_at: BlockNumber,
	pub key: PubKey,
    pub reserved_root: RootId,
    pub used_root: RootId,
	pub used_size: u64,
	pub reserved_size: u64,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct StatsInfo {
	pub used_size: u128,
	pub reserved_size: u128,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct FileOrder<AccountId, Balance, BlockNumber> {
	pub fee: Balance,
	pub expire_at: BlockNumber,
	pub replicas: Vec<AccountId>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct StoreFile<Balance> {
	pub reserved: Balance,
	pub file_size: u64,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct StashInfo<AccountId, Balance, BlockNumber> {
    pub stasher: AccountId,
	pub key: Option<PubKey>,
    pub deposit: Balance,
    pub claimed_round: RoundIndex,
	pub slash_defer_at: BlockNumber,
}

#[derive(PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct RoundRewardInfo<AccountId: Ord, Balance> {
	total_size: u128,
	individual: BTreeMap<AccountId, u64>,
    mine_reward: Balance,
    store_reward: Balance,
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: ReservableCurrency<Self::AccountId>;

		type UnixTime: UnixTime;

		type RoundPayout: RoundPayout<BalanceOf<Self>>;

		#[pallet::constant]
		type SlashDeferRounds: Get<u32>;

		#[pallet::constant]
		type SlashBalance: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type SlashRewardRatio: Get<Perbill>;

		#[pallet::constant]
		type RoundDuration: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type FileOrderRounds: Get<u32>;

		#[pallet::constant]
		type MaxFileReplicas: Get<u32>;

		#[pallet::constant]
		type FileBasePrice: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type FileBytePrice: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type StoreRewardRatio: Get<Perbill>;

		#[pallet::constant]
		type StashBalance: Get<BalanceOf<Self>>;

		#[pallet::constant]
        type HistoryRoundDepth: Get<u32>;
	}


	#[pallet::storage]
	pub type Enclaves<T: Config> = StorageMap<
		_,
		Twox64Concat,
		EnclaveId,
		BlockNumberFor<T>,
	>;

	#[pallet::storage]
	pub type StoragePotReserved<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub type Nodes<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		NodeInfo<BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	pub type Registers<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PubKey,
		EnclaveId,
	>;

	#[pallet::storage]
	pub type CurrentRound<T: Config> = StorageValue<_, RoundIndex, ValueQuery>;

	#[pallet::storage]
	pub type RoundsBlockNumber<T: Config> = StorageMap<
		_,
		Twox64Concat, RoundIndex,
		BlockNumberFor<T>,
        ValueQuery,
	>;

	#[pallet::storage]
	pub type RoundsReport<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat, RoundIndex,
		Blake2_128Concat, T::AccountId,
		bool, ValueQuery,
	>;

	#[pallet::storage]
	pub type Stats<T: Config> = StorageValue<_, StatsInfo, ValueQuery>;

	#[pallet::storage]
	pub type StoreFiles<T: Config> = StorageMap<
		_,
		Twox64Concat, RootId,
		StoreFile<BalanceOf<T>>,
	>;

	#[pallet::storage]
	pub type FileOrders<T: Config> = StorageMap<
		_,
		Twox64Concat, RootId,
		FileOrder<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	pub type Stashs<T: Config> = StorageMap<
		_,
		Blake2_128Concat, T::AccountId,
		StashInfo<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	pub type RoundsReward<T: Config> = StorageMap<
		_,
		Twox64Concat, RoundIndex,
		RoundRewardInfo<T::AccountId, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type RoundsStoreReward<T: Config> = StorageMap<
		_,
		Twox64Concat, RoundIndex,
		BalanceOf<T>, ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(
		T::AccountId = "AccountId",
		BalanceOf<T> = "Balance",
		PubKey = "PubKey",
		BlockNumberFor<T> = "BlockNumber",
	)]
	pub enum Event<T: Config> {
        SetEnclave(EnclaveId, BlockNumberFor<T>),
		NodeRegisted(T::AccountId, PubKey),
		NodeUpgraded(T::AccountId, PubKey),
		NodeReported(T::AccountId, PubKey),
        NodeWithdrawn(T::AccountId, BalanceOf<T>),
		StoreFileRequested(RootId, T::AccountId),
		StoreFileCharged(RootId, T::AccountId),
		StoreFileRemoved(RootId),
	}

	#[pallet::error]
	pub enum Error<T> {
        InvalidEnclaveExpire,
		InvalidStashPair,
		InvalidNode,
		InvalidStashKey,
		InvalidBase64Arg,
		InvalidIASSigningCert,
		InvalidIASBody,
		InvalidEnclave,
		InvalidReportBlock,
		InvalidVerifyP256Sig,
		IllegalSotrageReport,
		UnregisterNode,
		InvalidReportTime,
		InvalidReportSig,
		NodeUpgradeFailed,
		InvalidReportedNode,
		InvalidReportedData,
		NotEnoughReserved,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
            let next_round_bn = Self::get_next_round_bn();
			if now >= next_round_bn {
				Self::may_round_end();
			}
			// TODO: weights
			0
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub enclaves: Vec<(EnclaveId, BlockNumberFor<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				enclaves: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (code, bn) in &self.enclaves {
				Enclaves::<T>::insert(code.clone(), bn);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((1_000_000, DispatchClass::Operational))]
		pub fn set_enclave(
			origin: OriginFor<T>,
			enclave: EnclaveId,
			expire: T::BlockNumber,
		) -> DispatchResult {
            ensure_root(origin)?;
            if let Some(old_expire) = Enclaves::<T>::get(&enclave) {
                ensure!(expire < old_expire, Error::<T>::InvalidEnclaveExpire);
            }
            Enclaves::<T>::insert(&enclave, &expire);
            Self::deposit_event(Event::<T>::SetEnclave(enclave, expire));

            Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn stash(
			origin: OriginFor<T>,
			node: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stasher = ensure_signed(origin)?;
			let node = T::Lookup::lookup(node)?;
			let stash_balance = T::StashBalance::get();

			if let Some(mut stash_info) = Stashs::<T>::get(&node) {
				ensure!(&stash_info.stasher == &stasher, Error::<T>::InvalidStashPair);
				if stash_info.deposit >= stash_balance {
					stash_info.deposit = stash_info.deposit.saturating_sub(stash_balance);
				} else {
					let lack = stash_balance.saturating_sub(stash_info.deposit);
					T::Currency::transfer(&stasher, &Self::storage_pot(), lack, ExistenceRequirement::KeepAlive)?;
					stash_info.deposit = stash_balance;
				}
				Stashs::<T>::insert(node, stash_info);
			} else {
				T::Currency::transfer(&stasher, &Self::storage_pot(), stash_balance, ExistenceRequirement::KeepAlive)?;
				Stashs::<T>::insert(node, StashInfo {
					stasher,
					key: None,
					slash_defer_at: Zero::zero(),
					deposit: stash_balance,
                    claimed_round: CurrentRound::<T>::get(),
				});
			}
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn withdraw(
			origin: OriginFor<T>,
			node: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stasher = ensure_signed(origin)?;
			let node = T::Lookup::lookup(node)?;
            let mut stash_info = Stashs::<T>::get(&node).ok_or(Error::<T>::InvalidNode)?;
            ensure!(&stash_info.stasher == &stasher, Error::<T>::InvalidStashPair);
            let current_round = CurrentRound::<T>::get();
            let valid_round = current_round.saturating_sub(T::HistoryRoundDepth::get());
            let start_round = stash_info.claimed_round.max(valid_round);
            let mut total_mine_reward: BalanceOf<T> = Zero::zero();
            let mut total_store_reward: BalanceOf<T> = Zero::zero();
            for round in start_round..current_round {
                let reward_info =  RoundsReward::<T>::get(round);
                if let Some(individual) = reward_info.individual.get(&node) {
					let ratio = Perbill::from_rational(*individual as u128, reward_info.total_size);
                    let mine_reward =  ratio * reward_info.mine_reward;
					let store_reward  = ratio * reward_info.store_reward;
                    total_mine_reward = total_mine_reward.saturating_add(mine_reward);
					total_store_reward = total_store_reward.saturating_add(store_reward);
                }
            }
            if !total_mine_reward.is_zero() {
                T::Currency::deposit_creating(&Self::storage_pot(), total_mine_reward);
            }
            let new_deposit: BalanceOf<T> = stash_info.deposit.saturating_add(total_mine_reward).saturating_add(total_store_reward);
			let stash_balance = T::StashBalance::get();
            let free_amount = if new_deposit >= stash_balance {
				stash_info.deposit = stash_balance;
                new_deposit.saturating_sub(stash_balance)
            } else {
				stash_info.deposit = new_deposit;
                Zero::zero()
            };
            if !free_amount.is_zero() {
                T::Currency::transfer(&Self::storage_pot(), &stasher, free_amount, ExistenceRequirement::KeepAlive)?;
            }
            Stashs::<T>::insert(node.clone(), stash_info);
            Self::deposit_event(Event::<T>::NodeWithdrawn(node, free_amount));
            Ok(())
		}

		#[pallet::weight((1_000_000, DispatchClass::Operational))]
		pub fn register_node(
			origin: OriginFor<T>,
			cert: Vec<u8>,
			body: Vec<u8>,
			sig: Vec<u8>,
			p256_sig: Vec<u8>,
		) -> DispatchResult {
            let node = ensure_signed(origin)?;
			ensure!(Stashs::<T>::contains_key(&node), Error::<T>::InvalidNode);
			let dec_cert = base64::decode_config(&cert, base64::STANDARD).map_err(|_| Error::<T>::InvalidBase64Arg)?;
			let sig_cert = webpki::EndEntityCert::from(&dec_cert).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let dec_sig = base64::decode(&sig).map_err(|_| Error::<T>::InvalidBase64Arg)?;
			sig_cert.verify_signature(
				&webpki::RSA_PKCS1_2048_8192_SHA256,
				&body,
				&dec_sig
			).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let chain: Vec<&[u8]> = Vec::new();
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			let time_now = webpki::Time::from_seconds_since_unix_epoch(now);
			sig_cert.verify_is_valid_tls_server_cert(
				SUPPORTED_SIG_ALGS,
				&IAS_SERVER_ROOTS,
				&chain,
				time_now
			).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let json_body: serde_json::Value = serde_json::from_slice(&body).map_err(|_| Error::<T>::InvalidIASBody)?;
			if let serde_json::Value::String(isv_body) = &json_body["isvEnclaveQuoteBody"] {
				let isv_body = base64::decode(isv_body).map_err(|_| Error::<T>::InvalidIASBody)?;
				let now_at = Self::now_bn();
				let enclave = &isv_body[112..144].to_vec();
				ensure!(<Enclaves<T>>::iter().find(|(id, bn)| { bn > &now_at && id ==  enclave }).is_some(), Error::<T>::InvalidEnclave);
				let key = &isv_body[368..].to_vec();
				let data: Vec<u8> = [
					&cert[..],
					&sig[..],
					&body[..],
					&node.encode()[..],
				].concat();
				ensure!(verify_p256_sig(&key, &data, &p256_sig), Error::<T>::InvalidVerifyP256Sig);
				Registers::<T>::insert(key, enclave.clone());

				Self::deposit_event(Event::<T>::NodeRegisted(node, key.clone()))
			} else {
				return Err(Error::<T>::InvalidIASBody.into());
			}
            Ok(())
		}

		#[pallet::weight((1_000_000, DispatchClass::Operational))]
		pub fn report_files(
			origin: OriginFor<T>,
			key1: PubKey,
			key2: PubKey,
            bn: BlockNumberFor<T>,
            bh: Vec<u8>,
			reserved_size: u64,
			used_size: u64,
			reserved_root: RootId,
			used_root: RootId,
			sig: Vec<u8>,
			added_files: Vec<(RootId, u64, u64)>,
			deleted_files: Vec<(RootId, u64, u64)>,
			settle_files: Vec<RootId>,
			offline_nodes: Vec<T::AccountId>,
		) -> DispatchResult {
			let node = ensure_signed(origin)?;
            ensure!(
				reserved_size < RESERVED_SIZE_LIMIT && used_size < USED_SIZE_LIMIT && added_files.len() < FILES_COUNT_LIMIT,
				Error::<T>::IllegalSotrageReport
			);

			let mut stash_info = Stashs::<T>::get(&node).ok_or(Error::<T>::InvalidNode)?;

			ensure!(Self::verify_bn_and_bh(bn, &bh), Error::<T>::InvalidReportBlock);

			let enclave = Registers::<T>::try_get(&key1).map_err(|_| Error::<T>::UnregisterNode)?;
			let now_at = Self::now_bn();
			let enclave_bn = Enclaves::<T>::get(&enclave).ok_or(Error::<T>::InvalidEnclave)?;
			ensure!(now_at <= enclave_bn, Error::<T>::InvalidEnclave);

            let current_round = CurrentRound::<T>::get();
			let maybe_node_info: Option<NodeInfo<_>> = Nodes::<T>::get(&node);
			if let Some(_) = &maybe_node_info {
				if RoundsReport::<T>::get(current_round, &node) {
                    log!(
                        trace,
                        "🔒 Already reported with same pub key {:?} in the same slot {:?}.",
                        key1,
                        bn,
                    );
					return Ok(());
				}
			}
			ensure!(
				verify_report_storage(
					&key1,
					&key2,
					reserved_size,
					used_size,
					&added_files,
					&deleted_files,
					&reserved_root,
					&used_root,
					&sig,
				),
				Error::<T>::InvalidReportSig,
			);

			if !key2.is_empty() {
				// upgrade
				ensure!(Registers::<T>::contains_key(&key2), Error::<T>::NodeUpgradeFailed);
				let expect_key = stash_info.key.as_ref().ok_or(Error::<T>::InvalidStashKey)?;
				ensure!(expect_key == &key2, Error::<T>::InvalidStashKey);
				stash_info.key = Some(key1.clone());
				let node_info = maybe_node_info.ok_or(Error::<T>::NodeUpgradeFailed)?;
				ensure!(
					added_files.is_empty() &&
					deleted_files.is_empty() &&
					node_info.reserved_root == reserved_root &&
					node_info.used_root == used_root,
					Error::<T>::NodeUpgradeFailed
				);
				Registers::<T>::remove(&node_info.key);
				Self::deposit_event(Event::<T>::NodeUpgraded(node.clone(), key1.clone()));
			} else {
				if let Some(expect_key) = stash_info.key.as_ref() {
					ensure!(expect_key == &key1, Error::<T>::InvalidStashKey);
				} else {
					stash_info.key = Some(key1.clone());
				}
				if let Some(node_info) = &maybe_node_info {
					ensure!(&node_info.key == &key1, Error::<T>::InvalidReportedNode);
					let inc_size = added_files.iter().fold(0, |acc, (_, v, _)| acc + *v);
					let dec_size = deleted_files.iter().fold(0, |acc, (_, v, _)| acc + *v);
					let is_size_eq = if inc_size == 0 && dec_size == 0 {
						used_size == node_info.used_size
					} else {
						used_size == node_info.used_size.saturating_add(inc_size).saturating_sub(dec_size)
					};
					ensure!(is_size_eq, Error::<T>::InvalidReportedData);
				}
			}

			for (cid, file_size, ..) in added_files.iter() {
				Self::add_file(&node, cid, current_round, *file_size);
			}
			for (cid, ..) in deleted_files.iter() {
				Self::delete_file(&node, cid);
			}
			for cid in settle_files.iter() {
				Self::settle_file(&node, cid, current_round, &mut stash_info.deposit);
			}
			for offline_node in offline_nodes.iter() {
				Self::report_offline(&node, offline_node, current_round, now_at, &mut stash_info.deposit);
			}

			let new_node_info = NodeInfo {
				last_reported_at: now_at,
				key: key1.clone(),
				reserved_root,
				used_root,
				used_size,
				reserved_size,
			};

			RoundsReport::<T>::insert(current_round, node.clone(), true);
			Nodes::<T>::insert(node.clone(), new_node_info);
			Stashs::<T>::insert(node.clone(), stash_info);
			Self::deposit_event(Event::<T>::NodeReported(node, key1));
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn store_file(
			origin: OriginFor<T>,
			cid: RootId,
			file_size: u64,
			reserved: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			if let Some(mut file) = StoreFiles::<T>::get(&cid) {
				file.reserved = file.reserved.saturating_add(reserved);
				let min_reserved = Self::get_file_order_balance(file.file_size);
				ensure!(file.reserved >= min_reserved, Error::<T>::NotEnoughReserved);
                T::Currency::transfer(&who, &Self::storage_pot(), reserved, ExistenceRequirement::KeepAlive)?;
				StoreFiles::<T>::insert(cid.clone(), file);
				Self::deposit_event(Event::<T>::StoreFileCharged(cid, who));
			} else {
				let min_reserved = Self::get_file_order_balance(file_size);
				ensure!(reserved >= min_reserved, Error::<T>::NotEnoughReserved);
                T::Currency::transfer(&who, &Self::storage_pot(), reserved, ExistenceRequirement::KeepAlive)?;
				StoreFiles::<T>::insert(cid.clone(), StoreFile {
					reserved,
					file_size,
				});
				Self::deposit_event(Event::<T>::StoreFileRequested(cid, who));
			}
			Ok(())
		}
	}
}


impl<T: Config> Pallet<T> {

	pub fn storage_pot() -> T::AccountId {
		PALLET_ID.into_sub_account("stor")
	}

	fn may_round_end() {
        let current_round = CurrentRound::<T>::get();
        let next_round =  current_round.saturating_add(1);

        let to_remove_round = current_round.saturating_sub(T::HistoryRoundDepth::get());
		let mut stats: StatsInfo = Default::default();
		let mut individual_points: BTreeMap<T::AccountId, u64> = BTreeMap::new();


		for (ref controller, _) in RoundsReport::<T>::iter_prefix(&current_round) {
			if let Some(ref node_info) = Nodes::<T>::get(controller) {
				stats.used_size = stats.used_size.saturating_add(node_info.used_size as u128);
				stats.reserved_size = stats.reserved_size.saturating_add(node_info.reserved_size as u128);
				individual_points.insert(controller.clone(), node_info.used_size.saturating_add(node_info.reserved_size));
			}
		}
		let total_size = stats.used_size.saturating_add(stats.reserved_size);
		let mine_reward = T::RoundPayout::round_payout(total_size);
		let store_reward = RoundsStoreReward::<T>::get(current_round);

		RoundsReward::<T>::insert(current_round, RoundRewardInfo {
            mine_reward,
			store_reward,
			total_size,
			individual: individual_points,
		});
        RoundsBlockNumber::<T>::insert(next_round, Self::get_next_round_bn());
		CurrentRound::<T>::mutate(|v| *v = next_round);
		Stats::<T>::mutate(|v| *v = stats);
        Self::clear_round_information(to_remove_round);
	}

    fn clear_round_information(round: RoundIndex) {
        RoundsReport::<T>::remove_prefix(round, None);
        RoundsReward::<T>::remove(round);
		RoundsStoreReward::<T>::remove(round);
        RoundsBlockNumber::<T>::remove(round);
    }

	fn add_file(reporter: &T::AccountId, cid: &RootId, current_round: RoundIndex, file_size: u64) {
		if let Some(mut file_order) = FileOrders::<T>::get(cid) {
			let mut new_nodes = vec![];
			let mut exist = false;
			for node in file_order.replicas.iter() {
				if RoundsReport::<T>::get(current_round.saturating_sub(One::one()), node) {
					new_nodes.push(node.clone());
				}
				if node == reporter {
					exist = true;
				}
			}
			if !exist && (new_nodes.len() as u32) < T::MaxFileReplicas::get() {
				new_nodes.push(reporter.clone());
			}
			file_order.replicas = new_nodes;
			FileOrders::<T>::insert(cid, file_order);
		} else {
			Self::settle_file_order(cid, vec![reporter.clone()], Some(file_size))
		}
	}

	fn delete_file(reporter: &T::AccountId, cid: &RootId) {
		if let Some(mut file_order) = FileOrders::<T>::get(cid) {
			if let Ok(idx) = file_order.replicas.binary_search(reporter) {
				file_order.replicas.remove(idx);
				FileOrders::<T>::insert(cid, file_order);
			}
		}
	}

	fn settle_file(reporter: &T::AccountId, cid: &RootId, current_round: RoundIndex, new_reporter_deposit: &mut BalanceOf<T>) {
		if let Some(file_order) = FileOrders::<T>::get(cid) {
			if Self::now_bn() < file_order.expire_at {
				return;
			}
			let mut total_order_reward  = T::SlashRewardRatio::get() * file_order.fee;
			let each_order_reward = Perbill::from_rational(1, T::MaxFileReplicas::get()) * total_order_reward;
			for node in file_order.replicas.iter() {
				if let Some(mut stash_info) = Stashs::<T>::get(node) {
					if RoundsReport::<T>::get(current_round.saturating_sub(One::one()), node) {
						let mut order_reward = each_order_reward;
						if node == reporter {
							order_reward = order_reward.saturating_add(each_order_reward);
							*new_reporter_deposit = new_reporter_deposit.saturating_add(order_reward);
						} else {
							stash_info.deposit = stash_info.deposit.saturating_add(order_reward);
							Stashs::<T>::insert(node, stash_info);
						}
						total_order_reward = total_order_reward.saturating_sub(order_reward);
					}
				}
			}
			Self::settle_file_order(cid, file_order.replicas.clone(), None);
			let unpaid_reward = file_order.fee.saturating_sub(total_order_reward);
			RoundsStoreReward::<T>::mutate(current_round, |reward| *reward = reward.saturating_add(unpaid_reward));
		}
	}

	fn settle_file_order(cid: &RootId, nodes: Vec<T::AccountId>, file_size: Option<u64>) {
		if let Some(mut file) = StoreFiles::<T>::get(cid) {
			let expect_order_fee = Self::get_file_order_balance(file_size.unwrap_or(file.file_size));
			if let Some(file_size) = file_size {
				// user underreported the file size
				if file.file_size < file_size && file.reserved < expect_order_fee {
					StoragePotReserved::<T>::mutate(|reserved| *reserved = reserved.saturating_add(file.reserved));
					Self::clear_store_file(cid);
					return;
				}
				file.file_size = file_size;
			}
			let (mut order_fee, new_reserved) = if file.reserved > expect_order_fee {
				(expect_order_fee, file.reserved.saturating_sub(expect_order_fee))
			} else {
				(file.reserved, Zero::zero())
			};
			if order_fee.is_zero() {
				Self::clear_store_file(cid);
			} else {
				if order_fee < expect_order_fee {
					let lack_fee = expect_order_fee.saturating_sub(order_fee);
					let pot_reserved = StoragePotReserved::<T>::get();
					if pot_reserved > lack_fee {
						order_fee = expect_order_fee;
						StoragePotReserved::<T>::mutate(|reserved| *reserved = reserved.saturating_sub(lack_fee));
					} else {
						order_fee = order_fee.saturating_add(pot_reserved);
						StoragePotReserved::<T>::mutate(|reserved| *reserved = Zero::zero());
					}
				}
				FileOrders::<T>::insert(cid, FileOrder {
					fee: order_fee,
					expire_at: Self::get_file_order_expire(),
					replicas: nodes,
				});
				file.reserved = new_reserved;
				StoreFiles::<T>::insert(cid, file);
			}
		}
	}

	fn report_offline(reporter: &T::AccountId, offline_node: &T::AccountId, current_round: RoundIndex, now_at: BlockNumberFor<T>, new_reporter_deposit: &mut BalanceOf<T>) {
		if reporter == offline_node {
			return;
		}
		if RoundsReport::<T>::get(current_round.saturating_sub(One::one()), reporter) {
			return;
		}
		if let Some(mut stash_info) = Stashs::<T>::get(offline_node) {
			if now_at <= stash_info.slash_defer_at {
				return;
			}
			let slash_balance = T::SlashBalance::get();
			let (slash_reward, new_deposit) = if stash_info.deposit > slash_balance {
				(slash_balance, stash_info.deposit)
			} else {
				(stash_info.deposit, Zero::zero())
			};
			if slash_balance.is_zero() {
				return;
			}
			let reporter_reward = T::SlashRewardRatio::get() * slash_reward;
			let reserved_reward = slash_reward.saturating_sub(reporter_reward);
			*new_reporter_deposit = new_reporter_deposit.saturating_add(reporter_reward);

			stash_info.deposit = new_deposit;
			let slash_defer_bn = T::RoundDuration::get().saturating_mul(T::SlashDeferRounds::get().saturated_into());
			stash_info.slash_defer_at = stash_info.slash_defer_at.saturating_add(slash_defer_bn);
			Stashs::<T>::insert(offline_node, stash_info);
			StoragePotReserved::<T>::mutate(|reserved| *reserved = reserved.saturating_add(reserved_reward));
		}
	}

	fn clear_store_file(cid: &RootId) {
		StoreFiles::<T>::remove(cid);
		FileOrders::<T>::remove(cid);
		Self::deposit_event(Event::<T>::StoreFileRemoved(cid.clone()));
	}

	fn get_file_order_balance(file_size: u64) -> BalanceOf<T> {
		T::FileBytePrice::get().saturating_mul(file_size.saturated_into()).saturating_add(T::FileBasePrice::get())
	}

	fn get_file_order_expire() -> BlockNumberFor<T> {
		let now_at = Self::now_bn();
		let rounds = T::FileOrderRounds::get();
		now_at.saturating_add(T::RoundDuration::get().saturating_mul(rounds.saturated_into()))
	}

	fn verify_bn_and_bh(bn: BlockNumberFor<T>, bh: &Vec<u8>) -> bool {
        let hash = <frame_system::Pallet<T>>::block_hash(bn)
            .as_ref()
            .to_vec();
		if &hash != bh {
			return false;
		}
		bn == One::one() || bn == Self::get_current_round_bn()
	}

    fn get_current_round_bn() -> BlockNumberFor<T> {
        let current_round = CurrentRound::<T>::get();
        RoundsBlockNumber::<T>::get(current_round)
    }

    fn get_next_round_bn() -> BlockNumberFor<T> {
        let current_round = CurrentRound::<T>::get();
        RoundsBlockNumber::<T>::get(current_round) + T::RoundDuration::get()
    }

	fn now_bn() -> BlockNumberFor<T> {
		<frame_system::Pallet<T>>::block_number()
	}
}

pub fn verify_report_storage(
	key1: &PubKey,
	key2: &PubKey,
	reserved_size: u64,
	used_size: u64,
	added_files: &Vec<(RootId, u64, u64)>,
	deleted_files: &Vec<(RootId, u64, u64)>,
	reserved_root: &RootId,
	used_root: &RootId,
	sig: &Vec<u8>,
) -> bool {
	let data: Vec<u8> = [
		&key1[..],
		&key2[..],
		&encode_u64(reserved_size)[..],
		&encode_u64(used_size)[..],
		&reserved_root[..],
		&used_root[..],
		&encode_files(added_files)[..],
		&encode_files(deleted_files)[..],
	].concat();

	verify_p256_sig(key1, &data, sig)
}

pub fn verify_p256_sig(pk: &Vec<u8>, data: &Vec<u8>, sig: &Vec<u8>) -> bool {
    let mut pk = pk.clone();
    let mut sig = sig.clone();

    pk[0..32].reverse();
    pk[32..].reverse();

    sig[0..32].reverse();
    sig[32..].reverse();

    let vk: Vec<u8> = [
        &vec![4][..],
        &pk[..]
    ].concat();

	if let (Ok(sig), Ok(vk)) = (Signature::from_bytes(&sig), VerifyingKey::from_sec1_bytes(&vk[..])) {
		return vk.verify(data, &sig).is_ok()
	}
	false
}

pub fn encode_u64(number: u64) -> Vec<u8> {
    let mut value = number;
    let mut encoded_number: Vec<u8> = [].to_vec();
    loop {
        encoded_number.push((value%10) as u8 + 48u8); // "0" is 48u8
        value /= 10;
        if value == 0 {
            break;
        }
    }
    encoded_number.reverse();
    encoded_number
}

pub fn encode_files(fs: &Vec<(Vec<u8>, u64, u64)>) -> Vec<u8> {
    // "["
    let open_square_brackets_bytes: Vec<u8> = [91].to_vec();
    // "{\"cid\":\""
    let cid_bytes: Vec<u8> = [123, 34, 99, 105, 100, 34, 58, 34].to_vec();
    // "\",\"size\":"
    let size_bytes: Vec<u8> = [34, 44, 34, 115, 105, 122, 101, 34, 58].to_vec();
    // "}"
    let close_curly_brackets_bytes: Vec<u8> = [125].to_vec();
    // ","
    let comma_bytes: Vec<u8> = [44].to_vec();
    // "]"
    let close_square_brackets_bytes: Vec<u8> = [93].to_vec();
    let mut rst: Vec<u8> = open_square_brackets_bytes.clone();
    let len = fs.len();
    for (pos, (cid, size, ..)) in fs.iter().enumerate() {
        rst.extend(cid_bytes.clone());
        rst.extend(cid.clone());
        rst.extend(size_bytes.clone());
        rst.extend(encode_u64(*size));
        rst.extend(close_curly_brackets_bytes.clone());
        if pos != len-1 { rst.extend(comma_bytes.clone()) }
    }
    rst.extend(close_square_brackets_bytes.clone());
    rst
}

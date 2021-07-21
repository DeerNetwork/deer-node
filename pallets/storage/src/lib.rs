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
pub type MachineId = Vec<u8>;
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


#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
pub struct NodeInfo {
    pub rid: u64,
	pub used: u64,
	pub power: u64,
	pub last_round: RoundIndex,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
pub struct RewardInfo<Balance> {
	pub mine_reward: Balance,
	pub store_reward: Balance,
	pub paid_mine_reward: Balance,
	pub paid_store_reward: Balance,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct FileOrder<AccountId, Balance, BlockNumber> {
	pub fee: Balance,
	pub file_size: u64,
	pub expire_at: BlockNumber,
	pub replicas: Vec<AccountId>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct StoreFile<Balance> {
	pub reserved: Balance,
	pub base_reserved: Balance,
	pub file_size: u64,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct StashInfo<AccountId, Balance> {
    pub stasher: AccountId,
	pub register: Option<RegisterInfo>,
    pub deposit: Balance,
}


#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct RegisterInfo {
	pub key: PubKey,
	pub enclave: EnclaveId,
	pub machine_id: MachineId,
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
		type SlashBalance: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type RoundDuration: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type FileOrderRounds: Get<u32>;

		#[pallet::constant]
		type MaxFileReplicas: Get<u32>;

		#[pallet::constant]
		type MaxFileSize: Get<u64>;

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
		NodeInfo,
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
		(u64, u64), OptionQuery,
	>;

	#[pallet::storage]
	pub type RoundsSummary<T: Config> = StorageMap<
		_,
		Twox64Concat, RoundIndex,
		(u128, u128), ValueQuery,
	>;

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
		StashInfo<T::AccountId, BalanceOf<T>>,
	>;

	#[pallet::storage]
	pub type RoundsReward<T: Config> = StorageMap<
		_,
		Twox64Concat, RoundIndex,
		RewardInfo<BalanceOf<T>>, ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(
		T::AccountId = "AccountId",
		BalanceOf<T> = "Balance",
		MachineId = "MachineId",
		BlockNumberFor<T> = "BlockNumber",
	)]
	pub enum Event<T: Config> {
        SetEnclave(EnclaveId, BlockNumberFor<T>),
		NodeRegisted(T::AccountId, MachineId),
		NodeReported(T::AccountId, MachineId),
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
		MismatchMacheId,
		InvalidIASSign,
		InvalidIASSigningCert,
		InvalidIASBody,
		InvalidEnclave,
		InvalidVerifyP256Sig,
		IllegalReportFiles,
		UnregisterNode,
		InvalidReportSig,
		NotEnoughReserved,
		InvalidFileSize,
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
				CurrentRound::<T>::mutate(|v| *v = 1);
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
					register: None,
					deposit: stash_balance,
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
            let stash_deposit: BalanceOf<T> = stash_info.deposit;
			let stash_balance = T::StashBalance::get();
            let free_amount = if stash_deposit >= stash_balance {
				stash_info.deposit = stash_balance;
                stash_deposit.saturating_sub(stash_balance)
            } else {
				stash_info.deposit = stash_deposit;
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
		pub fn register(
			origin: OriginFor<T>,
			machine_id: MachineId,
			ias_cert: Vec<u8>,
			ias_sig: Vec<u8>,
			ias_body: Vec<u8>,
			sig: Vec<u8>,
		) -> DispatchResult {
            let node = ensure_signed(origin)?;
			let mut stash_info = Stashs::<T>::get(&node).ok_or(Error::<T>::InvalidNode)?;
			if let Some(register) = &stash_info.register {
				ensure!(&register.machine_id == &machine_id, Error::<T>::MismatchMacheId);
			}
			let dec_cert = base64::decode_config(&ias_cert, base64::STANDARD).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let sig_cert = webpki::EndEntityCert::from(&dec_cert).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let dec_sig = base64::decode(&ias_sig).map_err(|_| Error::<T>::InvalidIASSign)?;
			sig_cert.verify_signature(
				&webpki::RSA_PKCS1_2048_8192_SHA256,
				&ias_body,
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
			let json_body: serde_json::Value = serde_json::from_slice(&ias_body).map_err(|_| Error::<T>::InvalidIASBody)?;
			let isv_quote_body = json_body.get("isvEnclaveQuoteBody").and_then(|v| v.as_str()).ok_or(Error::<T>::InvalidIASBody)?;
			let isv_quote_body = base64::decode(isv_quote_body).map_err(|_| Error::<T>::InvalidIASBody)?;
			let now_at = Self::now_bn();
			let enclave = &isv_quote_body[112..144].to_vec();
			ensure!(<Enclaves<T>>::iter().find(|(id, bn)| { bn > &now_at && id ==  enclave }).is_some(), Error::<T>::InvalidEnclave);
			let key = &isv_quote_body[368..].to_vec();
			let data: Vec<u8> = [
				&ias_cert[..],
				&ias_sig[..],
				&ias_body[..],
				&machine_id[..],
			].concat();
			stash_info.register = Some(RegisterInfo {
				key: key.clone(),
				enclave: enclave.clone(),
				machine_id: machine_id.clone(),
			});
			ensure!(verify_p256_sig(&key, &data, &sig), Error::<T>::InvalidVerifyP256Sig);
			Stashs::<T>::insert(&node, stash_info);
			Self::deposit_event(Event::<T>::NodeRegisted(node, machine_id));
            Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn report(
			origin: OriginFor<T>,
			machine_id: MachineId,
			rid: u64,
			sig: Vec<u8>,
			added_files: Vec<(RootId, u64)>,
			deleted_files: Vec<RootId>,
			settle_files: Vec<RootId>,
		) -> DispatchResult {
			let reporter = ensure_signed(origin)?;
            ensure!(added_files.len() < FILES_COUNT_LIMIT, Error::<T>::IllegalReportFiles);
			let mut stash_info = Stashs::<T>::get(&reporter).ok_or(Error::<T>::InvalidNode)?;
			let register = stash_info.register.as_ref().ok_or(Error::<T>::UnregisterNode)?;
			ensure!(&register.machine_id == &machine_id, Error::<T>::MismatchMacheId);
			let now_at = Self::now_bn();
			let enclave_bn = Enclaves::<T>::get(&register.enclave).ok_or(Error::<T>::InvalidEnclave)?;
			let key = register.key.clone();
			ensure!(now_at <= enclave_bn, Error::<T>::InvalidEnclave);

            let current_round = CurrentRound::<T>::get();
			let maybe_node_info: Option<NodeInfo> = Nodes::<T>::get(&reporter);
			if let Some(_) = &maybe_node_info {
				if RoundsReport::<T>::contains_key(current_round, &reporter) {
                    log!(trace, "🔒 Already reported");
					return Ok(());
				}
			}
			let mut node_info = maybe_node_info.unwrap_or_default();

			ensure!(
				verify_report_storage(
					&machine_id,
					&key,
					node_info.rid,
					rid,
					&added_files,
					&deleted_files,
					&sig,
				),
				Error::<T>::InvalidReportSig,
			);

			let mut size_changed: BTreeMap<T::AccountId, (u64, u64)> = BTreeMap::new();

			for (cid, file_size, ..) in added_files.iter() {
				Self::add_file(&mut size_changed, &reporter, cid, current_round, *file_size);
			}
			for cid in deleted_files.iter() {
				Self::delete_file(&mut size_changed, &reporter, cid);
			}
			for cid in settle_files.iter() {
				Self::settle_file(&reporter, cid, current_round, &mut stash_info.deposit);
			}
			for (account, (size_added, size_deleted)) in size_changed.iter() {
				if account == &reporter {
					node_info.power = node_info.power.saturating_add(*size_added);
					node_info.used = node_info.used.saturating_add(*size_added).saturating_sub(*size_deleted);
				} else {
					Nodes::<T>::mutate(account, |maybe_node| {
						if let Some(other_node) = maybe_node {
							other_node.power = other_node.power.saturating_add(*size_added);
							other_node.used = other_node.used.saturating_add(*size_added).saturating_sub(*size_deleted);

						}
					})
				}
			}

			let prev_round = current_round.saturating_sub(One::one());
			if let Some(stats) = RoundsReport::<T>::get(prev_round, &reporter) {
				Self::round_reward(prev_round, stats, &mut stash_info);
			} else {
				if !node_info.last_round.is_zero() {
					Self::slash_offline(&mut stash_info);
				}
			}

			node_info.rid = rid;
			node_info.last_round = current_round;

			RoundsReport::<T>::insert(current_round, reporter.clone(),  (node_info.power, node_info.used));
			RoundsSummary::<T>::mutate(current_round, |(power, used)| {
				*power = power.saturating_add(node_info.power as u128);
				*used = used.saturating_add(node_info.used as u128);
			});
			Nodes::<T>::insert(reporter.clone(), node_info);
			Stashs::<T>::insert(reporter.clone(), stash_info);
			Self::deposit_event(Event::<T>::NodeReported(reporter, machine_id));
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn store(
			origin: OriginFor<T>,
			cid: RootId,
			file_size: u64,
			reserved: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(file_size > 0 && file_size <= T::MaxFileSize::get(), Error::<T>::InvalidFileSize);

			if let Some(mut file) = StoreFiles::<T>::get(&cid) {
				file.reserved = file.reserved.saturating_add(reserved);
				let min_reserved = Self::store_file_balance(file.file_size);
				ensure!(file.reserved >= min_reserved, Error::<T>::NotEnoughReserved);
                T::Currency::transfer(&who, &Self::storage_pot(), reserved, ExistenceRequirement::KeepAlive)?;
				StoreFiles::<T>::insert(cid.clone(), file);
				Self::deposit_event(Event::<T>::StoreFileCharged(cid, who));
			} else {
				let min_reserved = Self::store_file_balance(file_size);
				ensure!(reserved >= min_reserved, Error::<T>::NotEnoughReserved);
                T::Currency::transfer(&who, &Self::storage_pot(), reserved, ExistenceRequirement::KeepAlive)?;
				let base_reserved = T::FileBasePrice::get();
				StoreFiles::<T>::insert(cid.clone(), StoreFile {
					reserved: reserved.saturating_sub(base_reserved),
					base_reserved,
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
		let (power, _)= RoundsSummary::<T>::get(current_round);
		let mine_reward = T::RoundPayout::round_payout(power);
		if !mine_reward.is_zero() {
			T::Currency::deposit_creating(&Self::storage_pot(), mine_reward);
			RoundsReward::<T>::mutate(
				current_round, 
				|reward| {
					reward.mine_reward = mine_reward
				}
			);
		}
		// TODO collect dust reward to treasure

        RoundsBlockNumber::<T>::insert(next_round, Self::get_next_round_bn());
		CurrentRound::<T>::mutate(|v| *v = next_round);
        Self::clear_round_information(to_remove_round);
	}

    fn clear_round_information(round: RoundIndex) {
		if round.is_zero() { return; }
        RoundsReport::<T>::remove_prefix(round, None);
        RoundsBlockNumber::<T>::remove(round);
        // RoundsSummary::<T>::remove(round);
        // RoundsReward::<T>::remove(round);
    }

	fn add_file(
		size_changed: &mut BTreeMap<T::AccountId, (u64, u64)>,
		reporter: &T::AccountId,
		cid: &RootId,
		current_round: RoundIndex,
		file_size: u64,
	) {
		if let Some(mut file_order) = FileOrders::<T>::get(cid) {
			let mut new_nodes = vec![];
			let mut exist = false;
			for node in file_order.replicas.iter() {
				if RoundsReport::<T>::contains_key(current_round.saturating_sub(One::one()), node) {
					new_nodes.push(node.clone());
				} else {
					size_changed.get_mut(node).unwrap().1 = size_changed.get_mut(node).unwrap().1.saturating_add(file_size);
				}
				if node == reporter {
					exist = true;
				}
			}
			if !exist && (new_nodes.len() as u32) < T::MaxFileReplicas::get() {
				new_nodes.push(reporter.clone());
				size_changed.get_mut(reporter).unwrap().0 = size_changed.get_mut(reporter).unwrap().0.saturating_add(file_size);
			}
			file_order.replicas = new_nodes;
			FileOrders::<T>::insert(cid, file_order);
		} else {
			Self::settle_file_order(cid, vec![reporter.clone()], current_round, Some(file_size))
		}
	}

	fn delete_file(
		size_changed: &mut BTreeMap<T::AccountId, (u64, u64)>,
		reporter: &T::AccountId,
		cid: &RootId,
	) {
		if let Some(mut file_order) = FileOrders::<T>::get(cid) {
			if let Ok(idx) = file_order.replicas.binary_search(reporter) {
				file_order.replicas.remove(idx);
				size_changed.get_mut(reporter).unwrap().1 = size_changed.get_mut(reporter).unwrap().1.saturating_add(file_order.file_size);
				FileOrders::<T>::insert(cid, file_order);
			}
		}
	}

	fn settle_file(
		reporter: &T::AccountId,
		cid: &RootId,
		current_round: RoundIndex,
		new_reporter_deposit: &mut BalanceOf<T>,
	) {
		if let Some(file_order) = FileOrders::<T>::get(cid) {
			if Self::now_bn() < file_order.expire_at {
				return;
			}
			let mut total_order_reward  = T::StoreRewardRatio::get() * file_order.fee;
			let each_order_reward = Perbill::from_rational(1, T::MaxFileReplicas::get()) * total_order_reward;
			for node in file_order.replicas.iter() {
				if let Some(mut stash_info) = Stashs::<T>::get(node) {
					if RoundsReport::<T>::contains_key(current_round.saturating_sub(One::one()), node) {
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
			Self::settle_file_order(cid, file_order.replicas.clone(), current_round, None);
			let unpaid_reward = file_order.fee.saturating_sub(total_order_reward);
			RoundsReward::<T>::mutate(
				current_round, 
				|reward| {
					reward.store_reward = reward.store_reward.saturating_add(unpaid_reward)
				}
			);
		}
	}

	fn settle_file_order(
		cid: &RootId,
		nodes: Vec<T::AccountId>,
		current_round: RoundIndex,
		maybe_file_size: Option<u64>,
	) {
		if let Some(mut file) = StoreFiles::<T>::get(cid) {
			let expect_order_fee = Self::store_file_bytes_balance(maybe_file_size.unwrap_or(file.file_size));
			if let Some(file_size) = maybe_file_size {
				// user underreported the file size
				if file.file_size < file_size && file.reserved < expect_order_fee {
					StoragePotReserved::<T>::mutate(|reserved| *reserved = reserved.saturating_add(file.reserved));
					Self::clear_store_file(cid);
					return;
				}
				if !file.base_reserved.is_zero() {
					RoundsReward::<T>::mutate(
						current_round, 
						|reward| {
							reward.store_reward = reward.store_reward.saturating_add(file.base_reserved)
						}
					);
					file.base_reserved = Zero::zero();
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
					file_size: file.file_size,
					expire_at: Self::get_file_order_expire(),
					replicas: nodes,
				});
				file.reserved = new_reserved;
				StoreFiles::<T>::insert(cid, file);
			}
		}
	}

	fn round_reward(
		prev_round: RoundIndex,
		stats: (u64, u64),
		stash_info: &mut StashInfo<T::AccountId, BalanceOf<T>>,
	) {
		if prev_round.is_zero() {
			return;
		}
		let mut reward_info =  RoundsReward::<T>::get(prev_round);
		let (total_power, total_used) =  RoundsSummary::<T>::get(prev_round);
		let (power, used) = stats;
		let used_ratio = Perbill::from_rational(used as u128, total_used);
		let power_ratio = Perbill::from_rational(power as u128, total_power);
		let store_reward  = used_ratio * reward_info.store_reward;
		let mine_reward =  power_ratio * reward_info.mine_reward;
		reward_info.paid_store_reward = reward_info.paid_store_reward.saturating_add(store_reward);
		reward_info.paid_mine_reward = reward_info.paid_mine_reward.saturating_add(mine_reward);
		stash_info.deposit = stash_info.deposit.saturating_add(mine_reward).saturating_add(store_reward);
		RoundsReward::<T>::insert(prev_round, reward_info);
	}

	fn slash_offline(
		stash_info: &mut StashInfo<T::AccountId, BalanceOf<T>>,
	) {
		let slash_balance = T::SlashBalance::get();
		if slash_balance.is_zero() {
			return;
		}
		let (slash_reserved, new_deposit) = if stash_info.deposit > slash_balance {
			(slash_balance, stash_info.deposit)
		} else {
			(stash_info.deposit, Zero::zero())
		};
		stash_info.deposit = new_deposit;
		StoragePotReserved::<T>::mutate(|reserved| *reserved = reserved.saturating_add(slash_reserved));
	}

	fn clear_store_file(cid: &RootId) {
		StoreFiles::<T>::remove(cid);
		FileOrders::<T>::remove(cid);
		Self::deposit_event(Event::<T>::StoreFileRemoved(cid.clone()));
	}

	fn store_file_balance(file_size: u64) -> BalanceOf<T> {
		T::FileBasePrice::get().saturating_add(Self::store_file_bytes_balance(file_size))
	}

	fn store_file_bytes_balance(file_size: u64) -> BalanceOf<T> {
		let mut file_size_in_mega = file_size / 1_048_576;
		if file_size_in_mega % 1_048_576 != 0 {
			file_size_in_mega += 1;
		}
		T::FileBytePrice::get().saturating_mul(file_size_in_mega.saturated_into())
	}

	fn get_file_order_expire() -> BlockNumberFor<T> {
		let now_at = Self::now_bn();
		let rounds = T::FileOrderRounds::get();
		now_at.saturating_add(T::RoundDuration::get().saturating_mul(rounds.saturated_into()))
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
	machine_id: &MachineId,
	key: &PubKey,
	prev_rid: u64,
	rid: u64,
	added_files: &Vec<(RootId, u64)>,
	deleted_files: &Vec<RootId>,
	sig: &Vec<u8>,
) -> bool {
	let data: Vec<u8> = [
		&machine_id[..],
		&key[..],
		&encode_u64(prev_rid)[..],
		&encode_u64(rid)[..],
		&encode_add_files(added_files)[..],
		&encode_del_files(deleted_files)[..],
	].concat();

	verify_p256_sig(key, &data, sig)
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

fn encode_u64(number: u64) -> Vec<u8> {
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

fn encode_add_files(list: &Vec<(RootId, u64)>) -> Vec<u8> {
	let mut output = vec![];
    for (cid, size) in list.iter() {
		output.extend(cid.clone());
		output.extend(encode_u64(*size));
	}
	output
}

fn encode_del_files(list: &Vec<RootId>) -> Vec<u8> {
	let mut output = vec![];
    for cid in list.iter() {
		output.extend(cid.clone());
	}
	output
}
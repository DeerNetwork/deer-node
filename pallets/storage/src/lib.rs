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


use sp_std::{prelude::*, collections::btree_map::BTreeMap};
use sp_runtime::{Perbill, RuntimeDebug, SaturatedConversion, traits::{Zero, One, StaticLookup, Saturating, AccountIdConversion}};
use codec::{Encode, Decode};
use frame_support::{
	traits::{Currency, ReservableCurrency, ExistenceRequirement, UnixTime, Get},
};
use frame_system::{Config as SystemConfig, pallet_prelude::BlockNumberFor};
use p256::ecdsa::{VerifyingKey, signature::{Verifier, Signature}};

pub type FileId = Vec<u8>;
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
pub struct StoreFile<Balance, BlockNumber> {
	pub reserved: Balance,
	pub base_fee: Balance,
	pub file_size: u64,
	pub added_at: BlockNumber,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct StashInfo<AccountId, Balance> {
    pub stasher: AccountId,
    pub deposit: Balance,
	pub machine_id: Option<MachineId>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct RegisterInfo {
	pub key: PubKey,
	pub enclave: EnclaveId,
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
		type MaxReportFiles: Get<u32>;

		#[pallet::constant]
		type FileBaseFee: Get<BalanceOf<Self>>;

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
	pub type Registers<T: Config> = StorageMap<
		_,
		Twox64Concat,
		MachineId,
		RegisterInfo,
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
		Twox64Concat, FileId,
		StoreFile<BalanceOf<T>, BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	pub type FileOrders<T: Config> = StorageMap<
		_,
		Twox64Concat, FileId,
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
        Stashed(T::AccountId),
		NodeRegisted(T::AccountId, MachineId),
		NodeReported(T::AccountId, MachineId),
        Withdrawn(T::AccountId, BalanceOf<T>),
		StoreFileRequested(FileId, T::AccountId),
		StoreFileCharged(FileId, T::AccountId),
		StoreFileRemoved(FileId),
	}

	#[pallet::error]
	pub enum Error<T> {
        InvalidEnclaveExpire,
		InvalidStashPair,
		NoEnoughToWithdraw,
		InvalidNode,
		MismatchMacheId,
		MachineAlreadyRegistered,
		InvalidIASSign,
		InvalidIASSigningCert,
		InvalidIASBody,
		InvalidEnclave,
		DuplicateReport,
		InvalidVerifyP256Sig,
		IllegalReportFiles,
		UnregisterNode,
		InvalidReportSig,
		NotEnoughFee,
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
			controller: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stasher = ensure_signed(origin)?;
			let controller = T::Lookup::lookup(controller)?;
			let stash_balance = T::StashBalance::get();
			if let Some(mut stash_info) = Stashs::<T>::get(&controller) {
				ensure!(&stash_info.stasher == &stasher, Error::<T>::InvalidStashPair);
				if stash_info.deposit < stash_balance {
					let lack = stash_balance.saturating_sub(stash_info.deposit);
					T::Currency::transfer(&stasher, &Self::storage_pot(), lack, ExistenceRequirement::KeepAlive)?;
					stash_info.deposit = stash_balance;
					Stashs::<T>::insert(controller, stash_info);
				}
			} else {
				T::Currency::transfer(&stasher, &Self::storage_pot(), stash_balance, ExistenceRequirement::KeepAlive)?;
				Stashs::<T>::insert(&controller, StashInfo {
					stasher,
					deposit: stash_balance,
					machine_id: None,
				});
				Self::deposit_event(Event::<T>::Stashed(controller));
			}
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn withdraw(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
            let mut stash_info = Stashs::<T>::get(&controller).ok_or(Error::<T>::InvalidNode)?;
            let stash_deposit: BalanceOf<T> = stash_info.deposit;
			let stash_balance = T::StashBalance::get();
			let withdraw_balance = stash_deposit.saturating_sub(stash_balance);
			ensure!(!withdraw_balance.is_zero(), Error::<T>::NoEnoughToWithdraw);
			stash_info.deposit = stash_balance;
			T::Currency::transfer(&Self::storage_pot(), &stash_info.stasher, withdraw_balance, ExistenceRequirement::KeepAlive)?;
            Stashs::<T>::insert(controller.clone(), stash_info);
            Self::deposit_event(Event::<T>::Withdrawn(controller, withdraw_balance));
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
			let maybe_register_info = Registers::<T>::get(&machine_id);
			let mut stash_info = Stashs::<T>::get(&node).ok_or(Error::<T>::InvalidNode)?;
			if maybe_register_info.is_some() {
				ensure!(&stash_info.machine_id.is_some(), Error::<T>::MachineAlreadyRegistered);
			}
			if let Some(stash_machine_id) = &stash_info.machine_id {
				ensure!(stash_machine_id == &machine_id, Error::<T>::MismatchMacheId);
			}
			let dec_cert = base64::decode_config(&ias_cert, base64::STANDARD).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let sig_cert = webpki::EndEntityCert::from(&dec_cert).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let chain: Vec<&[u8]> = Vec::new();
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			let time_now = webpki::Time::from_seconds_since_unix_epoch(now);
			sig_cert.verify_is_valid_tls_server_cert(
				SUPPORTED_SIG_ALGS,
				&IAS_SERVER_ROOTS,
				&chain,
				time_now
			).map_err(|_| Error::<T>::InvalidIASSigningCert)?;
			let dec_sig = base64::decode(&ias_sig).map_err(|_| Error::<T>::InvalidIASSign)?;
			sig_cert.verify_signature(
				&webpki::RSA_PKCS1_2048_8192_SHA256,
				&ias_body,
				&dec_sig
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
			ensure!(verify_p256_sig(&key, &data, &sig), Error::<T>::InvalidVerifyP256Sig);

			match Registers::<T>::get(&machine_id) {
				Some(mut register) => {
					register.key = key.clone();
					register.enclave = enclave.clone();
					Registers::<T>::insert(&machine_id, register);
				},
				None => {
					Registers::<T>::insert(&machine_id, RegisterInfo {
						key: key.clone(),
						enclave: enclave.clone(),
					});
					stash_info.machine_id = Some(machine_id.clone());
					Stashs::<T>::insert(&node, stash_info);
				}
			}
			
			Self::deposit_event(Event::<T>::NodeRegisted(node, machine_id));
            Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn report(
			origin: OriginFor<T>,
			machine_id: MachineId,
			rid: u64,
			sig: Vec<u8>,
			add_files: Vec<(FileId, u64)>,
			del_files: Vec<FileId>,
			settle_files: Vec<FileId>
		) -> DispatchResult {
			let reporter = ensure_signed(origin)?;
            ensure!(
				add_files.len() < T::MaxReportFiles::get() as usize ||
				del_files.len() < T::MaxReportFiles::get() as usize ||
				settle_files.len() < T::MaxReportFiles::get() as usize, 
				Error::<T>::IllegalReportFiles
			);
			let mut stash_info = Stashs::<T>::get(&reporter).ok_or(Error::<T>::InvalidNode)?;
			ensure!(stash_info.machine_id.is_some(), Error::<T>::UnregisterNode);
			ensure!(&stash_info.machine_id.clone().unwrap() == &machine_id , Error::<T>::MismatchMacheId);
			let register = Registers::<T>::get(&machine_id).ok_or(Error::<T>::UnregisterNode)?;
			let now_at = Self::now_bn();
			let enclave_bn = Enclaves::<T>::get(&register.enclave).ok_or(Error::<T>::InvalidEnclave)?;
			ensure!(now_at <= enclave_bn, Error::<T>::InvalidEnclave);

            let current_round = CurrentRound::<T>::get();
			let prev_round = current_round.saturating_sub(One::one());
			let maybe_node_info: Option<NodeInfo> = Nodes::<T>::get(&reporter);
			if let Some(_) = &maybe_node_info {
				ensure!(!RoundsReport::<T>::contains_key(current_round, &reporter), Error::<T>::DuplicateReport);
			}
			let mut node_info = maybe_node_info.unwrap_or_default();

			ensure!(
				verify_report_storage(
					&machine_id,
					&register.key,
					node_info.rid,
					rid,
					&add_files,
					&del_files,
					&sig,
				),
				Error::<T>::InvalidReportSig,
			);

			let mut replica_changes: Vec<(T::AccountId, u64, bool)> = vec![];
			let mut current_round_reward = RoundsReward::<T>::get(current_round);
			let mut storage_pot_reserved = StoragePotReserved::<T>::get();
			let mut node_inc_deposits: BTreeMap<T::AccountId, BalanceOf<T>> = BTreeMap::new();
			let mut nodes_prev_reported: BTreeMap<T::AccountId, bool> = BTreeMap::new();

			for (cid, file_size, ..) in add_files.iter() {
				Self::add_file(
					&mut replica_changes,
					&mut current_round_reward.store_reward,
					&mut storage_pot_reserved,
					&mut nodes_prev_reported,
					&mut stash_info.deposit,
					&reporter,
					cid,
					prev_round,
					*file_size
				);
			}
			for cid in del_files.iter() {
				Self::delete_file(&mut replica_changes, &reporter, cid);
			}
			for cid in settle_files.iter() {
				Self::settle_file(
					&mut replica_changes,
					&mut current_round_reward.store_reward,
					&mut storage_pot_reserved,
					&mut stash_info.deposit,
					&mut node_inc_deposits,
					&mut nodes_prev_reported,
					&reporter,
					cid,
					prev_round
				);
			}

			if let Some(stats) = RoundsReport::<T>::get(prev_round, &reporter) {
				Self::round_reward(prev_round, stats, &mut stash_info);
			} else {
				if !node_info.last_round.is_zero() {
					Self::slash_offline(
						&mut storage_pot_reserved,
						&mut stash_info.deposit
					);
				}
			}

			let mut node_changes: BTreeMap<T::AccountId, (u64, u64)> = BTreeMap::new();
			for (account, file_size, is_add) in replica_changes.iter() {
				let (size_add, size_sub) = node_changes.entry(account.clone()).or_default();
				match *is_add {
					true => *size_add = size_add.saturating_add(*file_size),
					false => *size_sub = size_sub.saturating_sub(*file_size),
				}
			}

			let mut sumary = RoundsSummary::<T>::get(current_round);
			for (account, (size_inc, size_sub)) in node_changes.iter() {
				if account == &reporter {
					node_info.power = node_info.power.saturating_add(*size_inc);
					node_info.used = node_info.used.saturating_add(*size_inc).saturating_sub(*size_sub);
					sumary.0 = sumary.0.saturating_add(*size_inc as u128);
					sumary.1 = sumary.1.saturating_add(*size_inc as u128).saturating_sub(*size_sub as u128);
				} else {
					Nodes::<T>::mutate(account, |maybe_node| {
						if let Some(other_node) = maybe_node {
							other_node.power = other_node.power.saturating_add(*size_inc);
							other_node.used = other_node.used.saturating_add(*size_inc).saturating_sub(*size_sub);
							sumary.0 = sumary.0.saturating_add(*size_inc as u128);
							sumary.1 = sumary.1.saturating_add(*size_inc as u128).saturating_sub(*size_sub as u128);
						}
					})
				}
			}

			for (account, inc) in node_inc_deposits.iter() {
				Stashs::<T>::mutate(account, |maybe_stash_info| {
					if let Some(stash_info) = maybe_stash_info {
						stash_info.deposit = stash_info.deposit.saturating_add(*inc);
					} else {
						current_round_reward.store_reward = current_round_reward.store_reward.saturating_add(*inc);
					}
				})
			}

			node_info.rid = rid;
			node_info.last_round = current_round;

			StoragePotReserved::<T>::mutate(|v| *v = storage_pot_reserved);
			RoundsReward::<T>::insert(current_round, current_round_reward);
			RoundsReport::<T>::insert(current_round, reporter.clone(),  (node_info.power, node_info.used));
			RoundsSummary::<T>::mutate(current_round, |v|  *v = sumary);
			Nodes::<T>::insert(reporter.clone(), node_info);
			Stashs::<T>::insert(reporter.clone(), stash_info);
			Self::deposit_event(Event::<T>::NodeReported(reporter, machine_id));
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn store(
			origin: OriginFor<T>,
			cid: FileId,
			file_size: u64,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(file_size > 0 && file_size <= T::MaxFileSize::get(), Error::<T>::InvalidFileSize);

			if let Some(mut file) = StoreFiles::<T>::get(&cid) {
				let new_reserved = fee.saturating_add(file.reserved);
				let min_fee = Self::store_file_bytes_fee(file.file_size);
				ensure!(new_reserved >= min_fee, Error::<T>::NotEnoughFee);
                T::Currency::transfer(&who, &Self::storage_pot(), fee, ExistenceRequirement::KeepAlive)?;
				file.reserved = new_reserved;
				StoreFiles::<T>::insert(cid.clone(), file);
				Self::deposit_event(Event::<T>::StoreFileCharged(cid, who));
			} else {
				let min_fee = Self::store_file_fee(file_size);
				ensure!(fee >= min_fee, Error::<T>::NotEnoughFee);
                T::Currency::transfer(&who, &Self::storage_pot(), fee, ExistenceRequirement::KeepAlive)?;
				let base_fee = T::FileBaseFee::get();
				StoreFiles::<T>::insert(cid.clone(), StoreFile {
					reserved: fee.saturating_sub(base_fee),
					base_fee,
					file_size,
					added_at: Self::now_bn(), // TODO: file is invalid if no order for a long time
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
		replica_changes: &mut Vec<(T::AccountId, u64, bool)>,
		current_round_store_reward: &mut BalanceOf<T>,
		storage_pot_reserved: &mut BalanceOf<T>,
		nodes_prev_reported: &mut BTreeMap<T::AccountId, bool>,
		reporter_despoit: &mut BalanceOf<T>,
		reporter: &T::AccountId,
		cid: &FileId,
		prev_round: RoundIndex,
		file_size: u64,
	) {
		if let Some(mut file_order) = FileOrders::<T>::get(cid) {
			let mut new_nodes = vec![];
			let mut exist = false;
			for node in file_order.replicas.iter() {
				let reported = nodes_prev_reported.entry(node.clone()).or_insert_with(|| 
					Self::round_reported(prev_round, reporter)
				);
				if *reported {
					new_nodes.push(node.clone());
				} else {
					replica_changes.push((node.clone(), file_size, false));
				}
				if node == reporter {
					exist = true;
				}
			}
			if !exist && (new_nodes.len() as u32) < T::MaxFileReplicas::get() {
				new_nodes.push(reporter.clone());
				replica_changes.push((reporter.clone(), file_size, true));
			}
			file_order.replicas = new_nodes;
			FileOrders::<T>::insert(cid, file_order);
		} else {
			let ok = Self::settle_file_order(
				current_round_store_reward,
				storage_pot_reserved,
				reporter_despoit,
				cid, 
				vec![reporter.clone()], 
				Some(file_size)
			);
			if ok {
				replica_changes.push((reporter.clone(), file_size, true));
			}
		}
	}

	fn delete_file(
		replica_changes: &mut Vec<(T::AccountId, u64, bool)>,
		reporter: &T::AccountId,
		cid: &FileId,
	) {
		if let Some(mut file_order) = FileOrders::<T>::get(cid) {
			if let Ok(idx) = file_order.replicas.binary_search(reporter) {
				file_order.replicas.remove(idx);
				replica_changes.push((reporter.clone(), file_order.file_size, false));
				FileOrders::<T>::insert(cid, file_order);
			}
		}
	}

	fn settle_file(
		replica_changes: &mut Vec<(T::AccountId, u64, bool)>,
		current_round_store_reward: &mut BalanceOf<T>,
		storage_pot_reserved: &mut BalanceOf<T>,
		reporter_deposit: &mut BalanceOf<T>,
		node_inc_deposits: &mut BTreeMap<T::AccountId, BalanceOf<T>>,
		nodes_prev_reported: &mut BTreeMap<T::AccountId, bool>,
		reporter: &T::AccountId,
		cid: &FileId,
		prev_round: RoundIndex,
	) {
		if let Some(file_order) = FileOrders::<T>::get(cid) {
			if Self::now_bn() < file_order.expire_at {
				return;
			}
			let mut total_order_reward  = T::StoreRewardRatio::get() * file_order.fee;
			let each_order_reward = Perbill::from_rational(1, T::MaxFileReplicas::get()) * total_order_reward;
			for node in file_order.replicas.iter() {
				let reported = nodes_prev_reported.entry(node.clone()).or_insert_with(|| 
					Self::round_reported(prev_round, reporter)
				);
				if *reported {
					if node == reporter {
						*reporter_deposit = reporter_deposit.saturating_add(each_order_reward);
					} else {
						let node_deposit = node_inc_deposits.entry(node.clone()).or_default();
						*node_deposit = node_deposit.saturating_add(each_order_reward);
					}
					total_order_reward = total_order_reward.saturating_sub(each_order_reward);
				}
			}
			let ok = Self::settle_file_order(
				current_round_store_reward,
				storage_pot_reserved,
				reporter_deposit,
				cid, 
				file_order.replicas.clone(), 
				None
			);
			if !ok {
				for node in file_order.replicas.iter() {
					replica_changes.push((node.clone(), file_order.file_size, false));
				}
			}
			let unpaid_reward = file_order.fee.saturating_sub(total_order_reward);
			if !unpaid_reward.is_zero() {
				*current_round_store_reward = current_round_store_reward.saturating_add(unpaid_reward);
			}
		}
	}

	fn settle_file_order(
		current_round_store_reward: &mut BalanceOf<T>,
		storage_pot_reserved: &mut BalanceOf<T>,
		reporter_despoit: &mut BalanceOf<T>,
		cid: &FileId,
		nodes: Vec<T::AccountId>,
		maybe_file_size: Option<u64>,
	) -> bool {
		if let Some(mut file) = StoreFiles::<T>::get(cid) {
			let expect_order_fee = Self::store_file_bytes_fee(maybe_file_size.unwrap_or(file.file_size));
			if let Some(file_size) = maybe_file_size {
				if !file.base_fee.is_zero() {
					*storage_pot_reserved =  storage_pot_reserved.saturating_add(file.base_fee);
					// user underreported the file size
					if file.file_size < file_size && file.reserved < expect_order_fee {
						let to_reporter_reward = Perbill::from_rational(1, T::MaxFileReplicas::get()) * T::StoreRewardRatio::get() * file.reserved;
						*reporter_despoit = reporter_despoit.saturating_add(to_reporter_reward);
						*current_round_store_reward = current_round_store_reward.saturating_add(file.reserved.saturating_sub(to_reporter_reward));
						Self::clear_store_file(cid);
						return false;
					}
					file.base_fee = Zero::zero();
					file.file_size = file_size;
				}
			}
			let (mut order_fee, new_reserved) = if file.reserved > expect_order_fee {
				(expect_order_fee, file.reserved.saturating_sub(expect_order_fee))
			} else {
				(file.reserved, Zero::zero())
			};
			if order_fee.is_zero() {
				Self::clear_store_file(cid);
				return false;
			}
			if order_fee < expect_order_fee {
				let lack_fee = expect_order_fee.saturating_sub(order_fee);
				if *storage_pot_reserved > lack_fee {
					order_fee = expect_order_fee;
					*storage_pot_reserved = storage_pot_reserved.saturating_sub(lack_fee);
				} else {
					order_fee = order_fee.saturating_add(*storage_pot_reserved);
					*storage_pot_reserved = Zero::zero();
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
			true
		} else {
			false
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
		storage_pot_reserved: &mut BalanceOf<T>,
		reporter_deposit: &mut BalanceOf<T>,
	) {
		let slash_balance = T::SlashBalance::get();
		if slash_balance.is_zero() {
			return;
		}
		let (slash_reserved, new_deposit) = if *reporter_deposit > slash_balance {
			(slash_balance, *reporter_deposit)
		} else {
			(*reporter_deposit, Zero::zero())
		};
		*reporter_deposit = new_deposit;
		*storage_pot_reserved = storage_pot_reserved.saturating_add(slash_reserved);
	}

	fn round_reported(round: RoundIndex, node: &T::AccountId) -> bool {
		if round.is_zero() {
			return true;
		} 
		if RoundsReport::<T>::contains_key(round, node) {
			return true;
		}
		Nodes::<T>::get(&node).map(|v| v.last_round.is_zero()).unwrap_or_default()
	}

	fn clear_store_file(cid: &FileId) {
		StoreFiles::<T>::remove(cid);
		FileOrders::<T>::remove(cid);
		Self::deposit_event(Event::<T>::StoreFileRemoved(cid.clone()));
	}

	fn store_file_fee(file_size: u64) -> BalanceOf<T> {
		T::FileBaseFee::get().saturating_add(Self::store_file_bytes_fee(file_size))
	}

	fn store_file_bytes_fee(file_size: u64) -> BalanceOf<T> {
		let mut file_size_in_mega = file_size / 1_048_576;
		if file_size % 1_048_576 != 0 {
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
	added_files: &Vec<(FileId, u64)>,
	deleted_files: &Vec<FileId>,
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

fn encode_add_files(list: &Vec<(FileId, u64)>) -> Vec<u8> {
	let mut output = vec![];
    for (cid, size) in list.iter() {
		output.extend(cid.clone());
		output.extend(encode_u64(*size));
	}
	output
}

fn encode_del_files(list: &Vec<FileId>) -> Vec<u8> {
	let mut output = vec![];
    for cid in list.iter() {
		output.extend(cid.clone());
	}
	output
}
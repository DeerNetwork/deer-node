//! # Storage Online Module

#![cfg_attr(not(feature = "std"), no_std)]


// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
// #[cfg(test)]
// pub mod mock;
// #[cfg(test)]
// mod tests;

mod constants;
pub use constants::*;

// pub mod weights;


use sp_std::{
	prelude::*,
	collections::btree_map::BTreeMap,
	collections::btree_set::BTreeSet,
};
use sp_runtime::{
	RuntimeDebug, SaturatedConversion,
	traits::{Zero, One, StaticLookup, Saturating, AccountIdConversion}
};
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
pub struct OrderInfo<AccountId, Balance, BlockNumber> {
    pub file_size: u64,
	pub expire_at: Option<BlockNumber>,
    pub reserve: Balance,
    pub replicas: Vec<AccountId>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct StashInfo<AccountId, Balance> {
    pub stasher: AccountId,
    pub lock: Balance,
	pub free: Balance,
}

#[derive(PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct RoundRewardPoints<AccountId: Ord> {
	total: u128,
	individual: BTreeMap<AccountId, u64>,
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

		#[pallet::constant]
		type SlashDeferDuration: Get<u64>;

		#[pallet::constant]
		type RoundDuration: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type FileDuration: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type MaxFileReplica: Get<u32>;

		#[pallet::constant]
		type FilePrice: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxTrashSize: Get<u128>;

		#[pallet::constant]
		type MaxFileSize: Get<u64>;

		#[pallet::constant]
		type StashBalance: Get<BalanceOf<Self>>;
	}

	#[pallet::type_value]
	pub fn CurrentRoundAtOnEmpty<T: Config>() -> BlockNumberFor<T> { Zero::zero() }

	#[pallet::type_value]
	pub fn ReportedInRoundOnEmpty<T: Config>() -> BTreeSet<T::AccountId> {
		Default::default()
	}

	#[pallet::type_value]
	pub fn OnEmpty() -> StatsInfo {
		Default::default()
	}

	#[pallet::storage]
	pub type Enclaves<T: Config> = StorageMap<
		_,
		Twox64Concat,
		EnclaveId,
		BlockNumberFor<T>,
	>;

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
	pub type CurrentRoundAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery, CurrentRoundAtOnEmpty<T>>;

	#[pallet::storage]
	pub type ReportedInRound<T: Config> = StorageMap<
		_,
		Twox64Concat, BlockNumberFor<T>,
		BTreeSet<T::AccountId>,
		ValueQuery, ReportedInRoundOnEmpty<T>,
	>;

	#[pallet::storage]
	pub type Stats<T: Config> = StorageValue<_, StatsInfo, ValueQuery, OnEmpty>;

	#[pallet::storage]
	pub type Orders<T: Config> = StorageMap<
		_,
		Twox64Concat, RootId,
		OrderInfo<T::AccountId, BalanceOf<T>, T::BlockNumber>,
	>;

	#[pallet::storage]
	pub type Replicas<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat, T::AccountId,
		Twox64Concat, RootId,
		T::AccountId,
	>;

	#[pallet::storage]
	pub type Stashs<T: Config> = StorageMap<
		_,
		Blake2_128Concat, T::AccountId,
		StashInfo<T::AccountId, BalanceOf<T>>,
	>;

	#[pallet::storage]
	pub type RoundsRewardPoints<T: Config> = StorageMap<
		_,
		Twox64Concat, BlockNumberFor<T>,
		RoundRewardPoints<T::AccountId>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type RoundsReward<T: Config> = StorageMap<_, Twox64Concat, BlockNumberFor<T>, BalanceOf<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
	pub enum Event<T: Config> {
        SetEnclave(EnclaveId, T::BlockNumber),
		NodeRegisted(T::AccountId, PubKey),
		NodeUpgraded(T::AccountId, PubKey),
		NodeReported(T::AccountId, PubKey),
		OrderCreated(RootId, T::AccountId),
		OrderChanged(RootId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
        InvalidEnclaveExpire,
		InvalidStashPair,
		NotController,
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
		FileTooLarge,
		NotEnoughReserve,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> frame_support::weights::Weight {
			if (now % T::RoundDuration::get()).is_zero() {
				Self::may_round_end();
			}
			// TODO: weights
			0
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

			if let Some(ref info) = Stashs::<T>::get(&controller) {
				ensure!(&info.stasher == &stasher, Error::<T>::InvalidStashPair);
				let mut new_info = StashInfo {
					stasher: stasher.clone(),
					lock: Zero::zero(),
					free: Zero::zero(),
				};
				let total = info.lock.saturating_add(info.free);
				if total >= stash_balance {
					new_info.free = info.free.saturating_add(info.lock).saturating_sub(stash_balance);
					new_info.lock = stash_balance;
				} else {
					let lack = stash_balance.saturating_sub(total);
					T::Currency::transfer(&stasher, &Self::account_id(), lack, ExistenceRequirement::KeepAlive)?;
					new_info.lock = stash_balance;
				}
				Stashs::<T>::insert(controller, new_info);
			} else {
				T::Currency::transfer(&stasher, &Self::account_id(), stash_balance, ExistenceRequirement::KeepAlive)?;
				Stashs::<T>::insert(controller, StashInfo {
					stasher,
					lock: stash_balance,
					free: Zero::zero(),
				});
			}
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn reward_round(
			origin: OriginFor<T>,
			round: BlockNumberFor<T>,
			controller: T::AccountId,
		) -> DispatchResult {
			todo!()
		}

		#[pallet::weight(1_000_000)]
		pub fn register_node(
			origin: OriginFor<T>,
			cert: Vec<u8>,
			body: Vec<u8>,
			sig: Vec<u8>,
			p256_sig: Vec<u8>,
		) -> DispatchResult {
            let controller = ensure_signed(origin)?;
			ensure!(Stashs::<T>::contains_key(&controller), Error::<T>::NotController);
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
				let now_at = Self::get_now_bn();
				let enclave = &isv_body[112..144].to_vec();
				ensure!(<Enclaves<T>>::iter().find(|(id, bn)| { bn > &now_at && id ==  enclave }).is_some(), Error::<T>::InvalidEnclave);
				let key = &isv_body[368..].to_vec();
				let data: Vec<u8> = [
					&cert[..],
					&sig[..],
					&body[..],
					&controller.encode()[..],
				].concat();
				ensure!(verify_p256_sig(&key, &data, &p256_sig), Error::<T>::InvalidVerifyP256Sig);
				Registers::<T>::insert(key, enclave.clone());

				Self::deposit_event(Event::<T>::NodeRegisted(controller, key.clone()))
			} else {
				return Err(Error::<T>::InvalidIASBody.into());
			}
            Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn report_storage(
			origin: OriginFor<T>,
			key1: PubKey,
			key2: PubKey,
            bn: BlockNumberFor<T>,
            bh: Vec<u8>,
			reserved_size: u64,
			used_size: u64,
			added_files: Vec<(RootId, u64, u64)>,
			deleted_files: Vec<(RootId, u64, u64)>,
			reserved_root: RootId,
			used_root: RootId,
			sig: Vec<u8>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
            ensure!(
				reserved_size < SEAL_SIZE_LIMIT && used_size < FILES_SIZE_LIMIT && added_files.len() < FILES_COUNT_LIMIT,
				Error::<T>::IllegalSotrageReport
			);
			let enclave = Registers::<T>::try_get(&key1).map_err(|_| Error::<T>::UnregisterNode)?;
			let now_at = Self::get_now_bn();
			let enclave_bn = Enclaves::<T>::get(&enclave).ok_or(Error::<T>::InvalidEnclave)?;
			ensure!(now_at <= enclave_bn, Error::<T>::InvalidEnclave);

			let maybe_node_info: Option<NodeInfo<_>> = Nodes::<T>::get(&controller);
			if let Some(_) = &maybe_node_info {
				if ReportedInRound::<T>::get(bn).contains(&controller) {
                    log!(
                        trace,
                        "🔒 Already reported with same pub key {:?} in the same slot {:?}.",
                        key1,
                        bn,
                    );
					return Ok(());
				}
			}
			ensure!(Self::verify_bn_and_bh(bn, &bh), Error::<T>::InvalidReportBlock);
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
				let node_info = maybe_node_info.as_ref().ok_or(Error::<T>::NodeUpgradeFailed)?;
				ensure!(
					added_files.is_empty() &&
					deleted_files.is_empty() &&
					node_info.reserved_root == reserved_root &&
					node_info.used_root == used_root,
					Error::<T>::NodeUpgradeFailed
				);
				Registers::<T>::remove(&node_info.key);
				Self::deposit_event(Event::<T>::NodeUpgraded(controller.clone(), key1.clone()));
			} else {
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

			for (cid, ..) in added_files.iter() {
				Self::add_file(&controller, cid);
			}

			for (cid, ..) in deleted_files.iter() {
				Self::delete_file(&controller, cid);
			}

			let new_node_info = NodeInfo {
				last_reported_at: now_at,
				key: key1.clone(),
				reserved_root,
				used_root,
				used_size,
				reserved_size,
			};

			ReportedInRound::<T>::mutate(bn, |v| v.insert(controller.clone()));
			Nodes::<T>::insert(controller.clone(), new_node_info);
			Self::deposit_event(Event::<T>::NodeReported(controller, key1));
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn set_order(
			origin: OriginFor<T>,
			cid: RootId,
			file_size: u64,
			reserve: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(file_size < T::MaxFileSize::get(), Error::<T>::FileTooLarge);
			if let Some(mut order_info) = Orders::<T>::get(&cid) {
				order_info.reserve = order_info.reserve.saturating_add(reserve);
				Orders::<T>::insert(cid.clone(), order_info);
				Self::deposit_event(Event::<T>::OrderChanged(cid, who));
			} else {
				let min_reserve = Self::get_min_reserve(file_size);
				ensure!(reserve >= min_reserve, Error::<T>::NotEnoughReserve);
				Orders::<T>::insert(cid.clone(), OrderInfo {
					file_size,
					expire_at: None,
					reserve,
					replicas: vec![],
				});
				Self::deposit_event(Event::<T>::OrderCreated(cid, who));
			}
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn refresh_order(
			origin: OriginFor<T>,
			cid: RootId,
		) -> DispatchResult {
			todo!()
		}
	}
}


impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		PALLET_ID.into_account()
	}

	fn may_round_end() {
		let round_at = Self::round_bn(Self::get_now_bn());
		let current_round_at = CurrentRoundAt::<T>::get();

		if round_at == current_round_at {
			return;
		}

		let mut stats: StatsInfo = Default::default();
		let mut individual_points: BTreeMap<T::AccountId, u64> = BTreeMap::new();

		for controller in ReportedInRound::<T>::get(&current_round_at).iter() {
			if let Some(ref node_info) = Nodes::<T>::get(controller) {
				stats.used_size = stats.used_size.saturating_add(node_info.used_size as u128);
				stats.reserved_size = stats.reserved_size.saturating_add(node_info.reserved_size as u128);
				individual_points.insert(controller.clone(), node_info.used_size.saturating_add(node_info.reserved_size));
			}
		}

		// TODO: clear round_in_depth

		RoundsReward::<T>::insert(current_round_at, Self::calculate_round_reward(current_round_at));
		RoundsRewardPoints::<T>::insert(current_round_at, RoundRewardPoints {
			total: stats.used_size.saturating_add(stats.reserved_size),
			individual: individual_points,
		});
		Stats::<T>::mutate(|v| *v = stats);
		CurrentRoundAt::<T>::mutate(|v| *v = round_at);
	}

	fn add_file(account: &T::AccountId, cid: &RootId) {
		if let Some(mut order) = Orders::<T>::get(cid) {

		}
	}

	fn delete_file(account: &T::AccountId, cid: &RootId) {

	}

	fn get_min_reserve(file_size: u64) -> BalanceOf<T> {
		todo!()
	}

	fn calculate_round_reward(bn: BlockNumberFor<T>) -> BalanceOf<T> {
		todo!()
	}

	fn verify_bn_and_bh(bn: BlockNumberFor<T>, bh: &Vec<u8>) -> bool {
        let hash = <frame_system::Pallet<T>>::block_hash(bn)
            .as_ref()
            .to_vec();
		if &hash != bh {
			return false;
		}
		bn == One::one() || bn == Self::round_bn(Self::get_now_bn())
	}

	fn round_bn(bn: BlockNumberFor<T>) -> BlockNumberFor<T> {
		let round_duration = T::RoundDuration::get();
		let idx = bn / round_duration;
		idx * round_duration
	}

	fn get_now_bn() -> BlockNumberFor<T> {
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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		fail,
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement, OnUnbalanced, StorageVersion, WithdrawReasons},
		transactional,
	};
	use frame_system::pallet_prelude::*;
	pub use pallet_bridge as bridge;
	use scale_info::TypeInfo;
	use sp_arithmetic::traits::SaturatedConversion;
	use sp_core::U256;
	use sp_runtime::traits::StaticLookup;
	use sp_std::prelude::*;

	type ResourceId = bridge::ResourceId;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	pub struct AssetInfo {
		pub dest_id: bridge::BridgeChainId,
		pub asset_identity: Vec<u8>,
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + bridge::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Specifies the origin check provided by the bridge for calls that can only be called by
		/// the bridge pallet
		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// The currency mechanism.
		type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type NativeTokenResourceId: Get<ResourceId>;

		/// The handler to absorb the fee.
		type OnFeePay: OnUnbalanced<NegativeImbalanceOf<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Bridge fee updated.
		FeeUpdated { chain_id: bridge::BridgeChainId, min_fee: BalanceOf<T>, fee_scale: u32 },
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidTransfer,
		InvalidCommand,
		InvalidFeeOption,
		FeeOptionsMissing,
		InsufficientBalance,
		InvalidResourceId,
	}

	#[pallet::storage]
	#[pallet::getter(fn bridge_fee)]
	pub type BridgeFee<T: Config> =
		StorageMap<_, Twox64Concat, bridge::BridgeChainId, (BalanceOf<T>, u32), ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Change extra bridge transfer fee that user should pay
		#[pallet::weight(195_000_000)]
		pub fn change_fee(
			origin: OriginFor<T>,
			min_fee: BalanceOf<T>,
			fee_scale: u32,
			dest_id: bridge::BridgeChainId,
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			ensure!(fee_scale <= 1000u32, Error::<T>::InvalidFeeOption);
			BridgeFee::<T>::insert(dest_id, (min_fee, fee_scale));
			Self::deposit_event(Event::FeeUpdated { chain_id: dest_id, min_fee, fee_scale });
			Ok(())
		}

		/// Transfers some amount of the native token to some recipient on a (whitelisted)
		/// destination chain.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn transfer_native(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			recipient: Vec<u8>,
			dest_id: bridge::BridgeChainId,
		) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(<bridge::Pallet<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
			let bridge_id = <bridge::Pallet<T>>::account_id();
			ensure!(BridgeFee::<T>::contains_key(&dest_id), Error::<T>::FeeOptionsMissing);
			let fee = Self::calculate_fee(dest_id, amount);
			let free_balance = T::Currency::free_balance(&source);
			ensure!(free_balance >= (amount + fee), Error::<T>::InsufficientBalance);

			let imbalance = T::Currency::withdraw(
				&source,
				fee,
				WithdrawReasons::FEE,
				ExistenceRequirement::AllowDeath,
			)?;
			T::OnFeePay::on_unbalanced(imbalance);
			<T as Config>::Currency::transfer(
				&source,
				&bridge_id,
				amount,
				ExistenceRequirement::AllowDeath,
			)?;

			<bridge::Pallet<T>>::transfer_fungible(
				dest_id,
				T::NativeTokenResourceId::get(),
				recipient,
				U256::from(amount.saturated_into::<u128>()),
			)
		}

		/// Returns some amount of the native token as proposal rejected
		#[pallet::weight(195_000_000)]
		pub fn return_transfer_native(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			to: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			let to = T::Lookup::lookup(to)?;
			let bridge_id = <bridge::Pallet<T>>::account_id();
			<T as Config>::Currency::transfer(
				&bridge_id,
				&to,
				amount,
				ExistenceRequirement::AllowDeath,
			)?;
			Ok(())
		}

		/// Executes a simple currency transfer using the bridge account as the source
		#[pallet::weight(195_000_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
			rid: ResourceId,
		) -> DispatchResult {
			let source = T::BridgeOrigin::ensure_origin(origin)?;
			// transfer to bridge account from external accounts is not allowed.
			if source == to {
				fail!(Error::<T>::InvalidCommand);
			}

			if rid == T::NativeTokenResourceId::get() {
				// ERC20 DEER transfer
				<T as Config>::Currency::transfer(
					&source,
					&to,
					amount,
					ExistenceRequirement::AllowDeath,
				)?;
			} else {
				fail!(Error::<T>::InvalidResourceId);
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// TODO.wf: A more proper way to estimate fee
		pub fn calculate_fee(dest_id: bridge::BridgeChainId, amount: BalanceOf<T>) -> BalanceOf<T> {
			let (min_fee, fee_scale) = Self::bridge_fee(dest_id);
			let fee_estimated = amount * fee_scale.into() / 1000u32.into();
			if fee_estimated > min_fee {
				fee_estimated
			} else {
				min_fee
			}
		}
	}
}

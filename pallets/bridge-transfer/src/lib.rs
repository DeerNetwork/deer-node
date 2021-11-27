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
	use sp_runtime::traits::{CheckedAdd, CheckedSub};
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
		/// [chainId, min_fee, fee_scale]
		FeeUpdated(bridge::BridgeChainId, BalanceOf<T>, u32),
		/// [chainId, asset_identity, resource_id]
		AssetRegistered(bridge::BridgeChainId, Vec<u8>, bridge::ResourceId),
		/// [resource_id, amount]
		AssetMinted(bridge::ResourceId, BalanceOf<T>),
		/// [resource_id, amount]
		AssetBurned(bridge::ResourceId, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidTransfer,
		InvalidCommand,
		InvalidPayload,
		InvalidFeeOption,
		FeeOptionsMissing,
		InsufficientBalance,
		ResourceIdInUse,
		AssetNotRegistered,
		AccountNotExist,
		BalanceOverflow,
	}

	#[pallet::storage]
	#[pallet::getter(fn bridge_fee)]
	pub type BridgeFee<T: Config> =
		StorageMap<_, Twox64Concat, bridge::BridgeChainId, (BalanceOf<T>, u32), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bridge_assets)]
	pub type BridgeAssets<T: Config> = StorageMap<_, Twox64Concat, bridge::ResourceId, AssetInfo>;

	#[pallet::storage]
	#[pallet::getter(fn bridge_balances)]
	pub type BridgeBalances<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		bridge::ResourceId,
		Twox64Concat,
		T::AccountId,
		BalanceOf<T>,
	>;

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
			Self::deposit_event(Event::FeeUpdated(dest_id, min_fee, fee_scale));
			Ok(())
		}

		/// Register an asset.
		#[pallet::weight(195_000_000)]
		pub fn register_asset(
			origin: OriginFor<T>,
			asset_identity: Vec<u8>,
			dest_id: bridge::BridgeChainId,
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			let resource_id = bridge::derive_resource_id(
				dest_id,
				&bridge::hashing::blake2_128(&asset_identity.to_vec()),
			);
			ensure!(!BridgeAssets::<T>::contains_key(resource_id), Error::<T>::ResourceIdInUse);
			BridgeAssets::<T>::insert(
				resource_id,
				AssetInfo { dest_id, asset_identity: asset_identity.clone() },
			);
			Self::deposit_event(Event::AssetRegistered(dest_id, asset_identity, resource_id));
			Ok(())
		}

		/// Do mint operation on specific asset
		#[pallet::weight(195_000_000)]
		pub fn mint_asset(
			origin: OriginFor<T>,
			asset: bridge::ResourceId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			ensure!(BridgeAssets::<T>::contains_key(&asset), Error::<T>::AssetNotRegistered);
			let bridge_id = <bridge::Pallet<T>>::account_id();
			let holding_balance = BridgeBalances::<T>::get(&asset, &bridge_id).unwrap_or_default();
			BridgeBalances::<T>::insert(
				asset,
				&bridge_id,
				holding_balance.checked_add(&amount).ok_or(Error::<T>::BalanceOverflow)?,
			);
			Self::deposit_event(Event::AssetMinted(asset, amount));

			Ok(())
		}

		/// Do burn operation on specific asset
		#[pallet::weight(195_000_000)]
		pub fn burn_asset(
			origin: OriginFor<T>,
			asset: bridge::ResourceId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			ensure!(BridgeAssets::<T>::contains_key(&asset), Error::<T>::AssetNotRegistered);
			let bridge_id = <bridge::Pallet<T>>::account_id();
			let holding_balance = BridgeBalances::<T>::get(&asset, &bridge_id).unwrap_or_default();
			// check holding account balance to cover burn amount
			ensure!(
				Self::asset_balance(&asset, &bridge_id) >= amount,
				Error::<T>::InsufficientBalance
			);
			BridgeBalances::<T>::insert(
				asset,
				&bridge_id,
				holding_balance.checked_sub(&amount).ok_or(Error::<T>::BalanceOverflow)?,
			);
			Self::deposit_event(Event::AssetBurned(asset, amount));

			Ok(())
		}

		/// Transfer some amount of specific asset to some recipient on a (whitelisted) distination
		/// chain.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn transfer_assets(
			origin: OriginFor<T>,
			asset: bridge::ResourceId,
			amount: BalanceOf<T>,
			recipient: Vec<u8>,
			dest_id: bridge::BridgeChainId,
		) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(<bridge::Pallet<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
			ensure!(BridgeFee::<T>::contains_key(&dest_id), Error::<T>::FeeOptionsMissing);
			ensure!(BridgeAssets::<T>::contains_key(&asset), Error::<T>::AssetNotRegistered);
			// check account existence
			ensure!(
				BridgeBalances::<T>::contains_key(&asset, &source),
				Error::<T>::AccountNotExist
			);

			// check asset balance to cover transfer amount
			ensure!(
				Self::asset_balance(&asset, &source) >= amount,
				Error::<T>::InsufficientBalance
			);

			let fee = Self::calculate_fee(dest_id, amount);
			// check native balance to cover fee
			let native_free_balance = T::Currency::free_balance(&source);
			ensure!(native_free_balance >= fee, Error::<T>::InsufficientBalance);

			// pay fee to treasury
			let imbalance = T::Currency::withdraw(
				&source,
				fee,
				WithdrawReasons::FEE,
				ExistenceRequirement::AllowDeath,
			)?;
			T::OnFeePay::on_unbalanced(imbalance);

			// withdraw asset
			Self::do_asset_withdraw(&asset, &source, amount).ok_or(Error::<T>::BalanceOverflow)?;

			<bridge::Pallet<T>>::transfer_fungible(
				dest_id,
				asset,
				recipient,
				U256::from(amount.saturated_into::<u128>()),
			)
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

		//
		// Executable calls. These can be triggered by a bridge transfer initiated on another chain
		//

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
				// Other ERC20 token transfer
				ensure!(
					Self::asset_balance(&rid, &source) >= amount,
					Error::<T>::InsufficientBalance
				);
				Self::do_asset_deposit(&rid, &to, amount).ok_or(Error::<T>::BalanceOverflow)?;
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn asset_balance(asset: &bridge::ResourceId, who: &T::AccountId) -> BalanceOf<T> {
			BridgeBalances::<T>::get(asset, who).unwrap_or_default()
		}

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

		/// Deposit specific amount assets into recipient account.
		///
		/// Assets would be withdrawn from bridge account and then deposit to
		/// recipient.
		/// Bridge account is treat as holding account of all assets.
		///
		/// DO NOT guarantee asset was registered
		/// DO NOT guarantee bridge account(e.g. hodling account) has enough balance
		pub fn do_asset_deposit(
			asset: &bridge::ResourceId,
			recipient: &T::AccountId,
			amount: BalanceOf<T>,
		) -> Option<BalanceOf<T>> {
			let bridge_id = <bridge::Pallet<T>>::account_id();
			let holding_balance = BridgeBalances::<T>::get(asset, &bridge_id).unwrap_or_default();
			let recipient_balance = BridgeBalances::<T>::get(asset, recipient).unwrap_or_default();

			BridgeBalances::<T>::insert(asset, &bridge_id, holding_balance.checked_sub(&amount)?);
			BridgeBalances::<T>::insert(asset, recipient, recipient_balance.checked_add(&amount)?);

			Some(amount)
		}

		/// Withdraw specific amount assets from sender.
		///
		/// Assets would be withdrawn from the sender and then deposit to bridge account.
		/// Bridge account is treat as holding account of all assets.
		///
		/// DO NOT guarantee asset was registered
		/// DO NOT grarantee sender account has enough balance
		pub fn do_asset_withdraw(
			asset: &bridge::ResourceId,
			sender: &T::AccountId,
			amount: BalanceOf<T>,
		) -> Option<BalanceOf<T>> {
			let bridge_id = <bridge::Pallet<T>>::account_id();
			let holding_balance = BridgeBalances::<T>::get(asset, &bridge_id).unwrap_or_default();
			let recipient_balance = BridgeBalances::<T>::get(asset, sender).unwrap_or_default();

			BridgeBalances::<T>::insert(asset, sender, recipient_balance.checked_sub(&amount)?);
			BridgeBalances::<T>::insert(asset, &bridge_id, holding_balance.checked_add(&amount)?);

			Some(amount)
		}
	}
}

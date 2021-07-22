use super::*;
use crate::mock::*;
use frame_support::{assert_ok, assert_err, traits::Currency};


#[test]
fn set_enclave_works() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			assert_ok!(FileStorage::set_enclave(Origin::root(), mock_enclave_key2().0, 100));

			// should shorten period
			assert_ok!(FileStorage::set_enclave(Origin::root(), mock_enclave_key1().0, 100));

			// should not growth period
			assert_err!(
				FileStorage::set_enclave( Origin::root(), mock_enclave_key1().0, 100),
				Error::<Test>::InvalidEnclaveExpire
			);
		});
}


#[test]
fn stash_works() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			let stash_balance = default_stash_balance();
			let u1_b = Balances::free_balance(1);
			assert_ok!(FileStorage::stash(Origin::signed(1), 2));
			assert_eq!(Balances::free_balance(1), u1_b.saturating_sub(stash_balance));
			assert_eq!(balance_of_storage_pot(), stash_balance);
			assert_eq!(Stashs::<Test>::get(2).unwrap().deposit, stash_balance);

			// should recharge when account's stash_balance < T::StashBalance
			let stash_balance_x3 = stash_balance.saturating_mul(3);
			change_stash_balance(stash_balance_x3);
			assert_ok!(FileStorage::stash(Origin::signed(1), 2));
			assert_eq!(Balances::free_balance(1), u1_b.saturating_sub(stash_balance_x3));
			assert_eq!(Stashs::<Test>::get(2).unwrap().deposit, stash_balance_x3);
			assert_eq!(balance_of_storage_pot(), stash_balance_x3);

			// should do nothing when account's stash_balance > T::StashBalance
			change_stash_balance(stash_balance);
			assert_ok!(FileStorage::stash(Origin::signed(1), 2));
			assert_eq!(Balances::free_balance(1), u1_b.saturating_sub(stash_balance_x3));
			assert_eq!(Stashs::<Test>::get(2).unwrap().deposit, stash_balance_x3);
			assert_eq!(balance_of_storage_pot(), stash_balance_x3);

			// one stasher multiple controller
			assert_ok!(FileStorage::stash(Origin::signed(1), 3));

			// should not stash another controller
			assert_err!(
				FileStorage::stash(Origin::signed(11), 2), 
				Error::<Test>::InvalidStashPair,
			);
		})
}

#[test]
fn withdraw_works() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			let stash_balance = default_stash_balance();
			Balances::make_free_balance_be(&FileStorage::storage_pot(), 2_000);

			assert_ok!(FileStorage::stash(Origin::signed(1), 2));
			Stashs::<Test>::mutate(2, |maybe_stash_info| {
				if let Some(stash_info) = maybe_stash_info {
					stash_info.deposit = stash_info.deposit.saturating_add(1_000)
				}
			});
			let pot_b = balance_of_storage_pot();
			let u1_b = Balances::free_balance(1);
			assert_ok!(FileStorage::withdraw(Origin::signed(2)));
			assert_eq!(Stashs::<Test>::get(2).unwrap().deposit, stash_balance);
			assert_eq!(Balances::free_balance(1), u1_b.saturating_add(1_000));
			assert_eq!(balance_of_storage_pot(), pot_b.saturating_sub(1_000));

			// should not withdraw when account's deposit < T::StashBalance
			assert_ok!(FileStorage::stash(Origin::signed(11), 12));
			assert_err!(
				FileStorage::withdraw(Origin::signed(12)),
				Error::<Test>::NoEnoughToWithdraw,
			);
		})
}

#[test]
fn register_works() {
	ExtBuilder::default()
		.stash(1, 2)
		.build()
		.execute_with(|| {
			let register = mock_register1();
			assert_eq!(Stashs::<Test>::get(2).unwrap().register, None);
			assert_ok!(FileStorage::register(
				Origin::signed(2),
				register.machine_id,
				register.ias_cert,
				register.ias_sig,
				register.ias_body,
				register.sig
			));
			let register_info = Stashs::<Test>::get(2).unwrap().register.unwrap();
			assert_eq!(register_info.enclave, mock_enclave_key1().0);
			assert_eq!(register_info.key, mock_enclave_key1().1);
			assert_eq!(register_info.machine_id, mock_register1().machine_id);

			// register again with different register info
			let register = mock_register2();
			assert_ok!(FileStorage::register(
				Origin::signed(2),
				register.machine_id,
				register.ias_cert,
				register.ias_sig,
				register.ias_body,
				register.sig
			));
			let register_info = Stashs::<Test>::get(2).unwrap().register.unwrap();
			assert_eq!(register_info.enclave, mock_enclave_key2().0);
			assert_eq!(register_info.key, mock_enclave_key2().1);
			assert_eq!(register_info.machine_id, mock_register2().machine_id);

			// fail when controller is not bound
			let register = mock_register1();
			assert_err!(FileStorage::register(
				Origin::signed(3),
				register.machine_id,
				register.ias_cert,
				register.ias_sig,
				register.ias_body,
				register.sig
			), Error::<Test>::InvalidNode);

			// fail when machind_id don't match
			let register = mock_register1();
			let mut machine_id = register.machine_id;
			machine_id[0] += 1;
			assert_err!(FileStorage::register(
				Origin::signed(2),
				machine_id,
				register.ias_cert,
				register.ias_sig,
				register.ias_body,
				register.sig
			), Error::<Test>::MismatchMacheId);
		})
}

#[test]
fn report_works() {
	ExtBuilder::default()
		.stash(1, 2)
		.register(2, mock_register1())
		.build()
		.execute_with(|| {
			let reporter1 = mock_report1();
			assert_ok!(FileStorage::report(
				Origin::signed(2),
				reporter1.machine_id,
				reporter1.rid,
				reporter1.sig,
				reporter1.added_files,
				reporter1.deleted_files,
				reporter1.settle_files
			));
		})
}


#[test]
fn store_works() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			let pot_b = balance_of_storage_pot();
			let u1000_b = Balances::free_balance(&1000);
			let file_fee = 1100;
			assert_eq!(FileStorage::store_file_fee(2000), file_fee);
			assert_ok!(FileStorage::store(
				Origin::signed(1000),
				str2bytes("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeeeA"),
				100,
				file_fee,
			));
			let store_file = StoreFiles::<Test>::get(&str2bytes("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeeeA")).unwrap();
			assert_eq!(store_file, StoreFile { reserved: file_fee.saturating_sub(FILE_BASE_PRICE), base_fee: FILE_BASE_PRICE, file_size: 100 });
			assert_eq!(Balances::free_balance(&1000), u1000_b - file_fee);
			assert_eq!(balance_of_storage_pot(), pot_b.saturating_add(file_fee));

			// Add more fee to exist file pool
			assert_ok!(FileStorage::store(
				Origin::signed(1000),
				str2bytes("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeeeA"),
				1000,
				10,
			));
			let store_file = StoreFiles::<Test>::get(&str2bytes("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeeeA")).unwrap();
			assert_eq!(store_file, StoreFile { reserved: file_fee.saturating_sub(FILE_BASE_PRICE).saturating_add(10), base_fee: FILE_BASE_PRICE, file_size: 100 });


			// Fail when fee is not enough 
			assert_err!(FileStorage::store(
				Origin::signed(100),
				str2bytes("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeeeB"),
				100,
				file_fee.saturating_sub(1),
			), Error::<Test>::NotEnoughFee);


			// Fail when fize size not in [0, T::MaxFileSize]
			assert_err!(FileStorage::store(
				Origin::signed(1000),
				str2bytes("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeeeX"),
				MAX_FILE_SIZE + 1,
				u128::max_value(),
			), Error::<Test>::InvalidFileSize);
		})
}
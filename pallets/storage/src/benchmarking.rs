#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelist_account};
use frame_support::assert_ok;
use frame_system::{Pallet as System, RawOrigin as SystemOrigin};
use hex_literal::hex;
use sp_std::prelude::*;

use crate::Pallet as FileStorage;

const SEED: u32 = 0;
const FILE_ID_PREFIX: [u8; 43] = [
	81, 109, 83, 57, 69, 114, 68, 86, 120, 72, 88, 82, 78, 77, 74, 82, 74, 53, 105, 51, 98, 112,
	49, 122, 120, 67, 90, 122, 75, 80, 56, 81, 88, 88, 78, 72, 49, 121, 101, 101, 101, 101, 101,
];

fn get_enclave() -> Vec<u8> {
	hex!("f9895dfce305b1081c242421781364a49e7b54739cb7d2cf0bf578e4f393bfa3").into()
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

fn str2bytes(v: &str) -> Vec<u8> {
	v.as_bytes().to_vec()
}

fn create_funded_user<T: Config>(string: &'static str, balance_factor: u32) -> T::AccountId {
	let user = account(string, 0, SEED);
	let value = T::Currency::minimum_balance().saturating_mul(balance_factor.into());
	T::Currency::make_free_balance_be(&user, value);
	whitelist_account!(user);
	user
}

fn fund_storage_pot<T: Config>(balance_factor: u32) {
	let storage_pot = FileStorage::<T>::account_id();
	let value = T::Currency::minimum_balance().saturating_mul(balance_factor.into());
	T::Currency::make_free_balance_be(&storage_pot, value);
	whitelist_account!(storage_pot);
}

fn create_store_file<T: Config>(cid: &FileId, no_reserved: bool) {
	let reserved = if no_reserved {
		0u32.saturated_into()
	} else {
		FileStorage::<T>::store_file_bytes_fee(1_000_000)
	};
	StoreFiles::<T>::insert(
		cid.clone(),
		StoreFile {
			reserved,
			base_fee: T::FileBaseFee::get(),
			file_size: 1_000_000u64,
			added_at: 99u32.saturated_into(),
		},
	);
}

fn create_replica_nodes<T: Config>(
	num_replicas: u32,
	seed: u32,
	node: Option<T::AccountId>,
) -> Vec<T::AccountId> {
	let mut nodes = match node {
		Some(node) => vec![node],
		None => vec![],
	};
	for i in 0..num_replicas {
		let node: T::AccountId = account("replica", i, seed);
		Stashs::<T>::insert(
			node.clone(),
			StashInfo {
				stasher: account("replica", i, seed),
				deposit: T::Currency::minimum_balance().saturating_mul(1000u32.into()),
				machine_id: Some(vec![0u8; 16]),
			},
		);
		let mut node_info = NodeInfo::default();
		node_info.used = 10000000;
		node_info.power = 10000000000;
		Nodes::<T>::insert(node.clone(), node_info);
		nodes.push(node);
	}
	nodes
}

fn create_file_order<T: Config>(cid: &FileId, replicas: &[T::AccountId], settle: bool) {
	create_store_file::<T>(cid, settle);
	let expire_at: u32 = if settle { 999 } else { 1111 };
	FileOrders::<T>::insert(
		cid.clone(),
		FileOrder {
			fee: FileStorage::<T>::store_file_bytes_fee(1_000_000),
			file_size: 1_000_000,
			expire_at: expire_at.into(),
			replicas: replicas.to_vec(),
		},
	);
}

mod sign {
	use p256::ecdsa::{
		signature::{Signature, Signer},
		SigningKey,
	};
	use sp_std::prelude::*;
	pub fn p256_sign(
		machine_id: &[u8],
		priv_k: &[u8],
		pub_k: &[u8],
		prev_rid: u64,
		rid: u64,
		add_files: &[(Vec<u8>, u64)],
		del_files: &[Vec<u8>],
		power: u64,
	) -> Vec<u8> {
		let mut priv_k = priv_k.to_vec();
		priv_k.reverse();
		let sk = SigningKey::from_bytes(&priv_k).unwrap();
		let data = [
			&machine_id[..],
			&pub_k[..],
			&encode_u64(prev_rid)[..],
			&encode_u64(rid)[..],
			&encode_u64(power)[..],
			&encode_add_files(add_files)[..],
			&encode_del_files(del_files)[..],
		]
		.concat();
		let sigr = sk.sign(&data);
		let mut sig = sigr.as_bytes().to_vec();
		sig[0..32].reverse();
		sig[32..].reverse();
		sig
	}
	fn encode_u64(number: u64) -> Vec<u8> {
		let mut value = number;
		let mut encoded_number: Vec<u8> = [].to_vec();
		loop {
			encoded_number.push((value % 10) as u8 + 48u8); // "0" is 48u8
			value /= 10;
			if value == 0 {
				break
			}
		}
		encoded_number.reverse();
		encoded_number
	}

	fn encode_add_files(list: &[(Vec<u8>, u64)]) -> Vec<u8> {
		let mut output = vec![];
		for (cid, size) in list.iter() {
			output.extend(cid);
			output.extend(encode_u64(*size));
		}
		output
	}

	fn encode_del_files(list: &[Vec<u8>]) -> Vec<u8> {
		let mut output = vec![];
		for cid in list.iter() {
			output.extend(cid);
		}
		output
	}
}

benchmarks! {
	set_enclave {
		let enclave = get_enclave();
		let expire = 100u32.into();
	}: _(SystemOrigin::Root, enclave.clone(), expire)
	verify {
		assert_last_event::<T>(Event::<T>::SetEnclave(enclave, expire).into());
	}

	stash {
		let stasher = create_funded_user::<T>("stasher", 20000);
		let node: T::AccountId = account("node", 0, SEED);
		whitelist_account!(node);
		let node_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(node.clone());
	}: _(SystemOrigin::Signed(stasher.clone()), node_lookup)
	verify {
		assert_last_event::<T>(Event::<T>::Stashed(node).into());
	}

	withdraw {
		fund_storage_pot::<T>(20000u32.into());
		let stasher = create_funded_user::<T>("stasher", 20000);
		let node: T::AccountId = account("node", 0, SEED);
		whitelist_account!(node);
		let node_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(node.clone());
		assert_ok!(FileStorage::<T>::stash(SystemOrigin::Signed(stasher.clone()).into(), node_lookup));
		let value = T::Currency::minimum_balance().saturating_mul(10000u32.saturated_into());
		Stashs::<T>::mutate(&node, |maybe_stash| {
			if let Some(stash_info) = maybe_stash {
				stash_info.deposit = stash_info.deposit.saturating_add(value);
			}
		});
	}: _(SystemOrigin::Signed(node.clone()))
	verify {
		assert_last_event::<T>(Event::<T>::Withdrawn(node, stasher, value).into());
	}

	register {
		let enclave = get_enclave();
		assert_ok!(FileStorage::<T>::set_enclave(SystemOrigin::Root.into(), enclave, 1000000u32.into()));

		let stasher = create_funded_user::<T>("stasher", 20000);
		let node: T::AccountId = account("node", 0, SEED);
		whitelist_account!(node);
		let node_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(node.clone());
		assert_ok!(FileStorage::<T>::stash(SystemOrigin::Signed(stasher.clone()).into(), node_lookup));
		let machine_id: Vec<u8> = hex!("2663554671a5f2c3050e1cec37f31e55").into();
		let ias_body = str2bytes("{\"id\":\"327849746623058382595462695863525135492\",\"timestamp\":\"2021-07-21T07:23:39.696594\",\"version\":4,\"epidPseudonym\":\"ybSBDhwKvtRIx76tLCjLNVH+zI6JLGEEuu/c0mcQwk0OGYFRSsJfLApOkp+B/GFAzhTIIEXmYmAOSGDdbc2mFu/wx1HiK1+mFI+isaCe6ZN7IeLOrfbnVfeR6E7OhvFtc9e1xwyviVa6a9+bCVhQV1THJq7lW7HbaOxW9ZQu6g0=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00477\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00233\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"150200650400090000111102040180070000000000000000000C00000C000000020000000000000B2FD11FE6C355B3AB0F453E92C88F565CB58ACDCA00D3E13716CE6BDB92A372DA54784987293BE9EF77C00D94F090A9193BD6147A3C994E3086D14C57C089F35D39\",\"isvEnclaveQuoteBody\":\"AgABAC8LAAAMAAsAAAAAAAbkva5mzdO2S8iey0QRTKEAAAAAAAAAAAAAAAAAAAAABRICBf+AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAPmJXfzjBbEIHCQkIXgTZKSee1RznLfSzwv1eOTzk7+jAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH9m21/giIxl3atpQAIEkv0v5hVBPxPY2RMcR4xoxsgN+kc2W/n++sKQA8+PFpoHZis8WQdRHpnkOc3mnzlv+C\"}");
		let ias_sig = str2bytes("OcghuZnUiFmEs85hC0Ri2uJfyWR6lhhuCKY/U3UJTRee8GiENQCNj9dAQEYuUbUG4qEhdJeW4sM3RhV1MuOgYjut6UYXnhGXLDVg48ba+L+lDRQng+E26JYnQ0MOv0mMMJCNX1l3mHTUHM8e0C/kIWQJ+esuhR6G4WuHp7xyReZfJGbuKAkc6tC+q7e9XU9HvbSRaowjIfFMrXgJUZh5VG3Cj+6rDi807rL9oAxFTweivHiz6Tcvp3aZ7pH2QpDBL9OD68gwYfDxGvBi6+S1chqI7P6pFfWHcT+CISbOo2M6p9HpSVLf/07/9xxCrDU2/M5hDxSlVbXqKQKW2Mxt8A==");
		let ias_cert = str2bytes("MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==");
		let sig = hex!("90639853f8e815ede625c0b786c8453230790193aa5b29f5dca76e48845344503c8373a5cd9536d02504e0d74dfaef791af7f65e081a7be827f6d5e492424ca4").into();
	}: _(SystemOrigin::Signed(node.clone()), machine_id.clone(), ias_cert, ias_sig, ias_body, sig)
	verify {
		assert_last_event::<T>(Event::<T>::NodeRegisted(node, machine_id).into());
	}

	report {
		let x in 0..T::MaxFileReplicas::get();
		let y in 0..T::MaxFileReplicas::get();

		System::<T>::set_block_number(50000u32.into());

		let enclave = get_enclave();
		assert_ok!(FileStorage::<T>::set_enclave(SystemOrigin::Root.into(), enclave, 1000000u32.into()));

		let stasher = create_funded_user::<T>("stasher", 20000);
		let node: T::AccountId = account("node", 0, SEED);
		whitelist_account!(node);
		let node_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(node.clone());
		assert_ok!(FileStorage::<T>::stash(SystemOrigin::Signed(stasher.clone()).into(), node_lookup));
		let machine_id: Vec<u8> = hex!("2663554671a5f2c3050e1cec37f31e55").into();
		let ias_body = str2bytes("{\"id\":\"327849746623058382595462695863525135492\",\"timestamp\":\"2021-07-21T07:23:39.696594\",\"version\":4,\"epidPseudonym\":\"ybSBDhwKvtRIx76tLCjLNVH+zI6JLGEEuu/c0mcQwk0OGYFRSsJfLApOkp+B/GFAzhTIIEXmYmAOSGDdbc2mFu/wx1HiK1+mFI+isaCe6ZN7IeLOrfbnVfeR6E7OhvFtc9e1xwyviVa6a9+bCVhQV1THJq7lW7HbaOxW9ZQu6g0=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00477\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00233\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"150200650400090000111102040180070000000000000000000C00000C000000020000000000000B2FD11FE6C355B3AB0F453E92C88F565CB58ACDCA00D3E13716CE6BDB92A372DA54784987293BE9EF77C00D94F090A9193BD6147A3C994E3086D14C57C089F35D39\",\"isvEnclaveQuoteBody\":\"AgABAC8LAAAMAAsAAAAAAAbkva5mzdO2S8iey0QRTKEAAAAAAAAAAAAAAAAAAAAABRICBf+AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAPmJXfzjBbEIHCQkIXgTZKSee1RznLfSzwv1eOTzk7+jAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH9m21/giIxl3atpQAIEkv0v5hVBPxPY2RMcR4xoxsgN+kc2W/n++sKQA8+PFpoHZis8WQdRHpnkOc3mnzlv+C\"}");
		let ias_sig = str2bytes("OcghuZnUiFmEs85hC0Ri2uJfyWR6lhhuCKY/U3UJTRee8GiENQCNj9dAQEYuUbUG4qEhdJeW4sM3RhV1MuOgYjut6UYXnhGXLDVg48ba+L+lDRQng+E26JYnQ0MOv0mMMJCNX1l3mHTUHM8e0C/kIWQJ+esuhR6G4WuHp7xyReZfJGbuKAkc6tC+q7e9XU9HvbSRaowjIfFMrXgJUZh5VG3Cj+6rDi807rL9oAxFTweivHiz6Tcvp3aZ7pH2QpDBL9OD68gwYfDxGvBi6+S1chqI7P6pFfWHcT+CISbOo2M6p9HpSVLf/07/9xxCrDU2/M5hDxSlVbXqKQKW2Mxt8A==");
		let ias_cert = str2bytes("MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==");
		let sig = hex!("90639853f8e815ede625c0b786c8453230790193aa5b29f5dca76e48845344503c8373a5cd9536d02504e0d74dfaef791af7f65e081a7be827f6d5e492424ca4").into();

		assert_ok!(FileStorage::<T>::register(SystemOrigin::Signed(node.clone()).into(), machine_id.clone(), ias_cert, ias_sig, ias_body, sig));

		let mut add_files = vec![];
		for i in 0 .. x {
			let suffix: Vec<u8> = vec![65, 65 + i as u8 / 25,  65 + i as u8 % 25];
			let cid: FileId = [&FILE_ID_PREFIX[..], &suffix[..]].concat();
			if i == 0 {
				create_store_file::<T>(&cid, false);
			} else {
				let replicas = create_replica_nodes::<T>(10, 1000u32 + i as u32, None);
				create_file_order::<T>(&cid, &replicas, false);
			}
			add_files.push((cid, 1_000_000));
		}
		let mut del_files = vec![];
		for i in 0 .. y {
			let suffix: Vec<u8> = vec![66, 65 + i as u8 / 25,  65 + i as u8 % 25];
			let cid: FileId = [&FILE_ID_PREFIX[..], &suffix[..]].concat();
			let replicas = create_replica_nodes::<T>(T::MaxFileReplicas::get(), 2000u32 + i as u32, Some(node.clone()));
			create_file_order::<T>(&cid, &replicas, false);
			del_files.push(cid);
		}
		let settle_files = vec![];
		let rid = 1000;
		let power = 1000_000_000;
		let priv_k: Vec<u8> = hex!("e394cf1de366242a772f44904ba475f5317ce8baedac5485ccd812db2ccf28ab").into();
		let pub_k: Vec<u8> = hex!("87f66db5fe0888c65ddab6940020492fd2fe615413f13d8d9131c478c68c6c80dfa47365bf9fefac29003cf8f169a07662b3c5907511e99e439cde69f396ff82").into();

		let sig: Vec<u8> = sign::p256_sign(
			&machine_id,
			&priv_k,
			&pub_k,
			0,
			rid,
			&add_files,
			&del_files,
			power,
		);
	}: _(SystemOrigin::Signed(node.clone()), rid, power, sig, add_files, del_files, settle_files)
	verify {
		assert_last_event::<T>(Event::<T>::NodeReported(node, machine_id).into());
	}


	store {
		let cid = str2bytes("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeeeA");
		let user = create_funded_user::<T>("user", 10000);
		let fee = T::Currency::minimum_balance().saturating_mul(2000u32.saturated_into());
	}: _(SystemOrigin::Signed(user.clone()), cid.clone(), 100u64, fee)
	verify {
		assert_last_event::<T>(Event::<T>::StoreFileSubmitted(cid, user, fee).into());
	}

	force_delete {
		let cid = str2bytes("QmS9ErDVxHXRNMJRJ5i3bp1zxCZzKP8QXXNH1yeeeeeeeA");
		let user = create_funded_user::<T>("user", 10000);
		let fee = T::Currency::minimum_balance().saturating_mul(2000u32.saturated_into());
		assert_ok!(FileStorage::<T>::store(SystemOrigin::Signed(user.clone()).into(), cid.clone(), 100u64, fee));
		System::<T>::set_block_number(50000u32.into());
	}: _(SystemOrigin::Root, cid.clone())
	verify {
		assert_last_event::<T>(Event::<T>::FileForceDeleted(cid).into());
	}
}

impl_benchmark_test_suite!(
	FileStorage,
	crate::mock::ExtBuilder::default().enclaves(vec![]).build(),
	crate::mock::Test,
);

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

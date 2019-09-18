//! This file is a place holder for C wrappers
//! This is BLS's foreign function interface.

// structures
use crate::api::{BLSPKInG1, BLSPK, BLSSIG, BLSSK};
// constants
use crate::{PK_LEN, SIG_LEN, SK_LEN};
// traits
use api::BLSAPI;
use pairing::serdes::SerDes;

/// A wrapper of sk
#[repr(C)]
pub struct bls_sk {
    data: [u8; SK_LEN],
}
/// Implement Debug so clippy won't complain.
/// Not really used anywhere.
impl std::fmt::Debug for bls_sk {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (_i, e) in self.data.iter().enumerate() {
            write!(f, "{:02x}, ", e)?;
        }
        writeln!(f)
    }
}

/// A wrapper of pk
#[repr(C)]
pub struct bls_pk {
    data: [u8; PK_LEN],
}

/// Implement Debug so clippy won't complain.
/// Not really used anywhere.
impl std::fmt::Debug for bls_pk {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (_i, e) in self.data.iter().enumerate() {
            write!(f, "{:02x}, ", e)?;
        }
        writeln!(f)
    }
}

/// A wrapper that holds the output of key generation function.
#[repr(C)]
#[derive(Debug)]
pub struct bls_keys {
    pk: bls_pk,
    sk: bls_sk,
}

/// A wrapper of signature
#[repr(C)]
pub struct bls_sig {
    data: [u8; SIG_LEN],
}

/// Implement Debug so clippy won't complain.
/// Not really used anywhere.
impl std::fmt::Debug for bls_sig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (_i, e) in self.data.iter().enumerate() {
            write!(f, "{:02x}, ", e)?;
        }
        writeln!(f)
    }
}

/// Input a pointer to the seed, and its length, and a ciphersuie id.
/// The seed needs to be at least
/// 32 bytes long. Output the key pair.
/// Generate a pair of public keys and secret keys.
#[no_mangle]
pub unsafe extern "C" fn c_keygen(
    seed: *const u8,
    seed_len: libc::size_t,
    ciphersuite: u8,
) -> bls_keys {
    // convert a C array `seed` to a rust string `s`
    let s: &[u8] = std::slice::from_raw_parts(seed, seed_len as usize);

    // generate the keys
    let (sk, pk): (BLSSK, BLSPK) = BLSPKInG1::keygen(s, ciphersuite);

    // serialize the keys
    let mut pk_buf: Vec<u8> = vec![];
    assert!(
        pk.serialize(&mut pk_buf, true).is_ok(),
        "C wrapper error: keygen function: serializaing pk"
    );

    let mut sk_buf: Vec<u8> = vec![];
    assert!(
        sk.serialize(&mut sk_buf, true).is_ok(),
        "C wrapper error: keygen function: serializaing sk"
    );

    let mut pk_array = [0u8; PK_LEN];
    pk_array.copy_from_slice(&pk_buf);
    let mut sk_array = [0u8; SK_LEN];
    sk_array.copy_from_slice(&sk_buf);

    // return the keys
    bls_keys {
        pk: bls_pk { data: pk_array },
        sk: bls_sk { data: sk_array },
    }
}

/// Input a secret key, and a message in the form of a byte string,
/// output a signature.
#[no_mangle]
pub unsafe extern "C" fn c_sign(sk: bls_sk, msg: *const u8, msg_len: libc::size_t) -> bls_sig {
    // convert a C array `msg` to a rust string `m`
    let m: &[u8] = std::slice::from_raw_parts(msg, msg_len as usize);

    // load the secret key
    let mut k_buf = sk.data.to_vec();

    let (k, _compressed) = match BLSSK::deserialize(&mut k_buf[..].as_ref()) {
        Ok(p) => p,
        Err(e) => panic!("C wrapper error: signing function: deserialize sk: {}", e),
    };

    // generate the siganture, and return the pointer
    let sig = BLSPKInG1::sign(&k, m);

    // serialize the signature
    let mut sig_buf: Vec<u8> = vec![];
    assert!(
        sig.serialize(&mut sig_buf, true).is_ok(),
        "C wrapper error: signing function: serialize signature"
    );
    let mut sig_array = [0u8; SIG_LEN];
    sig_array.copy_from_slice(&sig_buf);
    bls_sig { data: sig_array }
}

/// Input a public key, a message in the form of a byte string,
/// and a signature, outputs true if signature is valid w.r.t. the inputs.
#[no_mangle]
pub unsafe extern "C" fn c_verify(
    pk: bls_pk,
    msg: *const u8,
    msglen: libc::size_t,
    sig: bls_sig,
) -> bool {
    // convert a C array `msg` to a rust string `m`
    let m: &[u8] = std::slice::from_raw_parts(msg, msglen as usize);

    // decompress the public key
    let mut k_buf = pk.data.to_vec();

    let (k, _compressed) = match BLSPK::deserialize(&mut k_buf[..].as_ref()) {
        Ok(p) => p,
        Err(e) => panic!(
            "C wrapper error: verification function: deserialize pk: {}",
            e
        ),
    };

    // decompress the signature
    let mut s_buf = sig.data.to_vec();
    let (s, _compressed) = match BLSSIG::deserialize(&mut s_buf[..].as_ref()) {
        Ok(p) => p,
        Err(e) => panic!(
            "C wrapper error: verification function: deserialize signature: {}",
            e
        ),
    };
    BLSPKInG1::verify(&k, m, &s)
}

/// This function aggregates the signatures without checking if a signature is valid or not.
/// It does check that all the signatures are for the same time stamp.
/// It panics if ciphersuite fails or time stamp is not consistent.
#[no_mangle]
pub unsafe extern "C" fn c_aggregation(sig_list: *mut bls_sig, sig_num: libc::size_t) -> Result<bls_sig, String> {
    let sig_list: &[bls_sig] = std::slice::from_raw_parts(sig_list as *mut bls_sig, sig_num);

    let mut sig_vec: Vec<BLSSIG> = vec![];

    for sig in sig_list.iter().take(sig_num) {
        // decompress the signature
        let (s, _compressed) = match BLSSIG::deserialize(&mut sig.data.as_ref()) {
            Ok(p) => p,
            Err(e) => panic!(
                "C wrapper error: signature aggregation function: deserialize signature: {}",
                e
            ),
        };

        sig_vec.push(s);
    }
    let agg_sig = BLSPKInG1::aggregate_without_verify(&sig_vec[..])?;

    let mut sig_buf: Vec<u8> = vec![];
    // serialize the updated sk
    assert!(
        agg_sig.serialize(&mut sig_buf, true).is_ok(),
        "C wrapper error: signature aggregation function: deserialize signature"
    );

    // return the aggregated signature
    let mut sig_array = [0u8; SIG_LEN];
    sig_array.copy_from_slice(&sig_buf);
    Ok(bls_sig { data: sig_array })
}

/// This function verifies the aggregated signature
#[no_mangle]
pub unsafe extern "C" fn c_verify_agg(
    pk_list: *mut bls_pk,
    pk_num: libc::size_t,
    msg: *const u8,
    msglen: libc::size_t,
    agg_sig: bls_sig,
) -> bool {
    let pk_list: &[bls_pk] = std::slice::from_raw_parts(pk_list as *mut bls_pk, pk_num);
    let mut pk_vec: Vec<BLSPK> = vec![];

    for pk in pk_list.iter().take(pk_num) {
        // decompress the signature
        let (s, _compressed) = match BLSPK::deserialize(&mut pk.data.as_ref()) {
            Ok(p) => p,
            Err(e) => panic!(
                "C wrapper error: signature aggregation function: deserialize signature: {}",
                e
            ),
        };

        pk_vec.push(s);
    }
    // convert a C array `msg` to a rust string `m`
    let m: &[u8] = std::slice::from_raw_parts(msg, msglen as usize);

    // decompress the signature
    let mut s_buf = agg_sig.data.to_vec();
    let (sig, _compressed) = match BLSSIG::deserialize(&mut s_buf[..].as_ref()) {
        Ok(p) => p,
        Err(e) => panic!(
            "C wrapper error: verification function: deserialize signature: {}",
            e
        ),
    };

    BLSPKInG1::verify_aggregated(pk_vec[..].as_ref(), m, &sig)
}

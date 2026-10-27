//! Ed25519 keys as OpenSSL writes them: PKCS#8 private (`openssl genpkey -algorithm ed25519`),
//! SPKI public (`openssl pkey -pubout`). Signatures are raw 64 bytes.
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};

const PRIVATE_DER: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];
const PUBLIC_DER: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

fn b64_value(c: u8) -> Option<u32> {
    Some(match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    } as u32)
}

pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let s: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !s.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::new();
    for chunk in s.chunks(4) {
        let pad = chunk.iter().rev().take_while(|b| **b == b'=').count();
        if pad > 2 || chunk[..4 - pad].contains(&b'=') {
            return None;
        }
        let mut n = 0u32;
        for c in &chunk[..4 - pad] {
            n = (n << 6) | b64_value(*c)?;
        }
        n <<= 6 * pad as u32;
        let bytes = n.to_be_bytes();
        out.extend_from_slice(&bytes[1..4 - pad]);
    }
    Some(out)
}

fn pem_body(text: &str, label: &str) -> Option<Vec<u8>> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = text.find(&begin)? + begin.len();
    let stop = start + text[start..].find(&end)?;
    base64_decode(&text[start..stop])
}

fn key32(der: &[u8], prefix: &[u8]) -> Option<[u8; 32]> {
    (der.len() == prefix.len() + 32 && der.starts_with(prefix))
        .then(|| der[prefix.len()..].try_into().ok())?
}

/// 32-byte public key from a `PUBLIC KEY` PEM.
pub fn public_from_pem(text: &str) -> Result<[u8; 32], String> {
    pem_body(text, "PUBLIC KEY")
        .and_then(|d| key32(&d, &PUBLIC_DER))
        .ok_or_else(|| "not an Ed25519 PUBLIC KEY PEM".into())
}

/// 32-byte seed from a `PRIVATE KEY` PEM.
pub fn seed_from_pem(text: &str) -> Result<[u8; 32], String> {
    pem_body(text, "PRIVATE KEY")
        .and_then(|d| key32(&d, &PRIVATE_DER))
        .ok_or_else(|| "not an Ed25519 PRIVATE KEY PEM (PKCS#8)".into())
}

pub fn public_of(seed: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(seed).verifying_key().to_bytes()
}

pub fn sign(seed: &[u8; 32], msg: &[u8]) -> [u8; 64] {
    use ed25519_dalek::Signer;
    SigningKey::from_bytes(seed).sign(msg).to_bytes()
}

pub fn verify(public: &[u8; 32], msg: &[u8], sig: &[u8]) -> bool {
    let (Ok(k), Ok(s)) = (
        VerifyingKey::from_bytes(public),
        <[u8; 64]>::try_from(sig).map(|b| Signature::from_bytes(&b)),
    ) else {
        return false;
    };
    k.verify_strict(msg, &s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // RFC 8032 section 7.1, test 1, as OpenSSL writes the keys.
    const PRIV: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g\n-----END PRIVATE KEY-----\n";
    const PUB: &str = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=\n-----END PUBLIC KEY-----\n";

    #[test]
    fn rfc8032_vector_through_pem() {
        let seed = seed_from_pem(PRIV).unwrap();
        assert_eq!(
            seed.to_vec(),
            hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        );
        let pk = public_from_pem(PUB).unwrap();
        assert_eq!(public_of(&seed), pk);
        let sig = sign(&seed, b"");
        assert_eq!(
            sig.to_vec(),
            hex(
                "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
            )
        );
        assert!(verify(&pk, b"", &sig));
        assert!(!verify(&pk, b"x", &sig));
        assert!(!verify(&pk, b"", &sig[..63]));
    }

    #[test]
    fn pem_rejects_table() {
        for bad in [
            "",
            PUB.replace("PUBLIC", "PRIVATE").as_str(),
            "-----BEGIN PRIVATE KEY-----\n!!!!\n-----END PRIVATE KEY-----",
            "-----BEGIN PRIVATE KEY-----\nMC4CAQAw\n-----END PRIVATE KEY-----",
        ] {
            assert!(seed_from_pem(bad).is_err(), "{bad}");
        }
        assert!(public_from_pem(PRIV).is_err());
    }

    #[test]
    fn base64_table() {
        for (s, want) in [
            ("", Some(&b""[..])),
            ("Zg==", Some(b"f")),
            ("Zm8=", Some(b"fo")),
            ("Zm9v", Some(b"foo")),
            ("Zm9v\nYmFy", Some(b"foobar")),
            ("Zm9", None),
            ("Z===", None),
            ("Zg=a", None),
        ] {
            assert_eq!(base64_decode(s).as_deref(), want, "{s}");
        }
    }
}

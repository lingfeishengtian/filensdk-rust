use openssl::hash::{self, MessageDigest};
use openssl::pkcs5;

use crate::responses::auth::AuthVersion;

const DEFAULT_PBKDF2_ROUNDS: u32 = 200000;
const DEFAULT_PBKDF2_BIT_LENGTH: u32 = 512;

pub enum PkdbHashLevels {
    SHA1,
    SHA256,
    SHA512,
}

fn pbkdf2(password: &str, salt: &str, bit_length: u32, hash: PkdbHashLevels, rounds: u32) -> String {
    let digest = match hash {
        PkdbHashLevels::SHA1 => MessageDigest::sha1(),
        PkdbHashLevels::SHA256 => MessageDigest::sha256(),
        PkdbHashLevels::SHA512 => MessageDigest::sha512(),
    };

    // Bit shift right 3 to divide by 8
    let mut out = vec![0u8; (bit_length >> 3) as usize];
    pkcs5::pbkdf2_hmac(password.as_bytes(), salt.as_bytes(), rounds as usize, digest, &mut out).unwrap();

    hex::encode(out)
}

pub struct DerivedCredentials {
    pub master_key: String,
    pub password: String,
}

pub fn derive_credentials_from_password(
    auth_version: AuthVersion,
    password: &str,
    salt: Option<&str>,
) -> DerivedCredentials {
    if let AuthVersion::V1 = auth_version {
        println!("V1 is unsupported");

        DerivedCredentials {
            master_key: "".to_owned(),
            password: "".to_owned(),
        }
    } else {
        if salt.is_none() {
            panic!("Salt is required for AuthVersion V2");
        }

        let salt = salt.unwrap();
        let derived_key_string = pbkdf2(password, salt, DEFAULT_PBKDF2_BIT_LENGTH, PkdbHashLevels::SHA512, DEFAULT_PBKDF2_ROUNDS);
        
        let middle_index = derived_key_string.len() / 2;
        let (master_key, password) = derived_key_string.split_at(middle_index);

        let hashed_password = hash::hash(MessageDigest::sha512(), password.as_bytes()).unwrap();
        let hashed_password = hex::encode(hashed_password);

        DerivedCredentials {
            master_key: master_key.to_owned(),
            password: hashed_password,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbkdf2() {
        let salt_hashed = pbkdf2(
            "test", 
            "saltyasthesea", 
            DEFAULT_PBKDF2_BIT_LENGTH, 
            PkdbHashLevels::SHA512, 
            DEFAULT_PBKDF2_ROUNDS
        );

        assert_eq!(salt_hashed, "215624a1a33f9962aa2e4a6beeade36dca74a300bece1981c984db32fff85692fc4077d7a7b8f0f69d25ce49c3b0455f5e888a87c80a1f2d7e8bcce99a67fc4a");
    }

    #[test]
    fn test_derive_credentials_from_password() {
        let derived_credentials = derive_credentials_from_password(
            AuthVersion::V2,
            "test",
            Some("saltyasthesea"),
        );

        assert_eq!(derived_credentials.master_key, "215624a1a33f9962aa2e4a6beeade36dca74a300bece1981c984db32fff85692");
        assert_eq!(derived_credentials.password, "d103ae8e5fec137e5586bf75707b274b07b8d2ab607d63ac75fb586e8dff9d691ddc104426ce2f9225d3d785b6bffebd9b0c7c579ca5fd53aad0b4808f20e57d");

        let derived_credentials = derive_credentials_from_password(
            AuthVersion::V2,
            "test",
            Some("test"),
        );

        assert_eq!(derived_credentials.master_key, "8809fd1f1e620cf1156353571199e227adeb766ab435c9fa0d0cb3097f5d8fdf");
        assert_eq!(derived_credentials.password, "61da3afe761a9bfe7cdc7db9783ed2fdb12157eed2be209db0fc3c17b8396bb3e0fc6844b01c5ca7a605861c6a792669d10e76a4b002d68d3e8cdedfeb167893");
    }
}
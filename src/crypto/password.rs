use std::num::NonZero;
use crate::responses::auth::AuthVersion;

const DEFAULT_PBKDF2_ROUNDS: u32 = 200000;
const DEFAULT_PBKDF2_BIT_LENGTH: u32 = 512;
const DEFAULT_PBKDF2_ARR_SIZE: usize = (DEFAULT_PBKDF2_BIT_LENGTH >> 3) as usize;

pub enum PkdbHashLevels {
    SHA1,
    SHA256,
    SHA512,
}

#[derive(uniffi::Record)]
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
        let mut out = vec![0u8; DEFAULT_PBKDF2_ARR_SIZE];
        ring::pbkdf2::derive(ring::pbkdf2::PBKDF2_HMAC_SHA512, NonZero::new(DEFAULT_PBKDF2_ROUNDS).unwrap(), salt.as_bytes(), password.as_bytes(), &mut out);
        let derived_key_string = hex::encode(out);
        
        let middle_index = derived_key_string.len() / 2;
        let (master_key, password) = derived_key_string.split_at(middle_index);

        // let hashed_password = hash::hash(MessageDigest::sha512(), (password).as_bytes()).unwrap();
        let hashed_password = ring::digest::digest(&ring::digest::SHA512, password.as_bytes());
        let hashed_password = hex::encode(hashed_password);

        DerivedCredentials {
            master_key: master_key.to_string(),
            password: hashed_password,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
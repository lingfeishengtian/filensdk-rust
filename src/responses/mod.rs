pub mod auth;
pub mod fs;

use serde::Deserialize;

#[macro_export]
macro_rules! response_struct {
    // Match multiple struct definitions with optional attributes for fields
    ($( $name:ident { $($(#[$meta:meta])* $field:ident: $type:ty,)* } )*) => {
        $(
            #[derive(uniffi::Record)]
            #[derive(Deserialize, Debug)]
            #[serde(rename_all = "camelCase")]
            pub struct $name {
                $(
                    $(#[$meta])*
                    pub $field: $type,
                )*
            }
        )*
    };
}

// Support function for serde to parse boolean values from ints
fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let i = i32::deserialize(deserializer)?;
    Ok(i != 0)
}
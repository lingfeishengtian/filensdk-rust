pub mod auth;
pub mod fs;

#[macro_export]
macro_rules! request_struct {
    // Match multiple struct definitions
    ($( $name:ident { $($field:ident: $type:ty,)* } )*) => {
        $(
            #[derive(uniffi::Record)]
            #[derive(Serialize, Debug)]
            #[serde(rename_all = "camelCase")]
            pub struct $name {
                $(pub $field: $type,)*
            }
        )*
    };
}
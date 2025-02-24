use serde::Serialize;

use crate::{request_struct, responses::auth::AuthVersion};

request_struct! {
    LoginRequest {
        email: String,
        password: String,
        two_factor_code: String,
        auth_version: AuthVersion,
    }

    AuthInfoRequest {
        email: String,
    }
}

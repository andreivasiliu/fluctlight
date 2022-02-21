/*
#[derive(Serialize, Deserialize)]
pub(crate) struct RequestBody<'a> {
    #[serde(borrow)]
    pub device_keys: BTreeMap<&'a Id<User>, Vec<&'a Id<Device>>>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Response<'a> {
    #[serde(borrow)]
    pub device_keys: BTreeMap<&'a Id<User>, BTreeMap<&'a Id<Device>, DeviceKeys<'a>>>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct DeviceKeys<'a> {
    #[serde(borrow)]
    pub user_id: &'a Id<User>,
    #[serde(borrow)]
    pub device_id: &'a Id<Device>,
    #[serde(borrow)]
    pub algorithms: Vec<EventEncryptionAlgorithm<'a>>,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum EventEncryptionAlgorithm<'a> {
    OlmV1Curve25519AesSha2,
    MegolmV1AesSha2,
    Other(&'a str),
}

impl<'a> MatrixRequest for RequestBody<'a> {
    type Response = Response<'a>;
}

// POST /_matrix/federation/v2/user/keys/query
pub(super) fn post_federation_v2_user_keys_query<'r>(
    request_data: &RequestData<'r>,
    _request: rest_api_types::post_federation_v1_user_keys_query::Request<'r>,
) -> rest_api_types::post_federation_v1_user_keys_query::Response<'r> {
    let mut device_keys = BTreeMap::new();

    for (user, user_state) in &request_data.state.users {
        let mut user_device_keys = BTreeMap::new();

        for (device, _key) in &user_state.keys {
            user_device_keys.insert(&**device, rest_api_types::post_federation_v1_user_keys_query::DeviceKeys {
                user_id: &**user, device_id: &**device, algorithms: vec![],
            });

        }
        device_keys.insert(&**user, user_device_keys);
    }

    rest_api_types::post_federation_v1_user_keys_query::Response {
        device_keys,
    }
}
*/

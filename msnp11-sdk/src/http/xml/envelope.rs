use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct SoapEnvelope {
    #[serde(rename = "Body")]
    pub soap_body: SoapBody,
}

#[derive(Deserialize)]
pub(crate) struct SoapBody {
    #[serde(rename = "GetClientConfigResponse")]
    pub get_client_config_response: GetClientConfigResponse,
}

#[derive(Deserialize)]
pub(crate) struct GetClientConfigResponse {
    #[serde(rename = "GetClientConfigResult")]
    pub get_client_config_result: String,
}

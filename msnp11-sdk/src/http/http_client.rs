use crate::errors::sdk_error::SdkError;
#[cfg(feature = "config")]
use crate::http::config::Config;
#[cfg(feature = "config")]
use crate::http::xml::envelope::SoapEnvelope;
#[cfg(feature = "config")]
use crate::http::xml::msgr_config::MsgrConfig;
use reqwest::header::{AUTHORIZATION, HeaderMap};
#[cfg(feature = "config")]
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use std::error::Error;

pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn get_login_srf(&self, nexus_url: &str) -> Result<String, Box<dyn Error>> {
        let response = self.client.get(nexus_url).send().await?;
        let mut url = response
            .headers()
            .get("Passporturls")
            .ok_or(SdkError::AuthenticationHeaderNotFound)?
            .to_str()?
            .replace("DALogin=", "");

        if !url.starts_with("http") {
            url.insert_str(0, "https://");
        }

        Ok(url)
    }

    pub async fn get_passport_token(
        &self,
        email: &str,
        password: &str,
        nexus_url: &str,
        authorization_string: &str,
    ) -> Result<String, SdkError> {
        let login_srf = self
            .get_login_srf(nexus_url)
            .await
            .or(Err(SdkError::CouldNotGetAuthenticationString))?;

        let mut headers = HeaderMap::with_capacity(1);
        headers.insert(AUTHORIZATION, format!("Passport1.4 OrgVerb=GET,OrgURL=http%3A%2F%2Fmessenger%2Emsn%2Ecom,sign-in={email},pwd={password},{authorization_string}").parse().or(Err(SdkError::CouldNotGetAuthenticationString))?);

        let response = self
            .client
            .get(login_srf)
            .headers(headers)
            .send()
            .await
            .or(Err(SdkError::ReceivingError))?;

        let authentication_info = response
            .headers()
            .get("Authentication-Info")
            .ok_or(SdkError::AuthenticationHeaderNotFound)?
            .to_str()
            .or(Err(SdkError::CouldNotGetAuthenticationString))?;

        let mut token = authentication_info.split("from-PP='");
        token.next();

        let token = token
            .next()
            .ok_or(SdkError::CouldNotGetAuthenticationString)?;

        Ok(token.replace("'", ""))
    }

    #[cfg(feature = "config")]
    pub async fn get_config(&self, config_url: &str) -> Result<Config, Box<dyn Error>> {
        let mut headers = HeaderMap::with_capacity(2);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/xml"));
        headers.insert(
            "SOAPAction",
            HeaderValue::from_static(
                "http://www.msn.com/webservices/Messenger/Client/GetClientConfig",
            ),
        );

        let xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
        <soap:Envelope xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\
                       xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\"\
                       xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\">
            <soap:Body>
                <GetClientConfig xmlns='http://www.msn.com/webservices/Messenger/Client'>
                    <clientinfo>
                        <Country>00</Country>
                        <CLCID>0809</CLCID>
                        <PLCID>0409</PLCID>
                        <GeoID>32</GeoID>
                    </clientinfo>
                </GetClientConfig>
            </soap:Body>
        </soap:Envelope>";

        let response = self
            .client
            .post(config_url)
            .headers(headers)
            .body(xml)
            .send()
            .await?;

        let xml = response.text().await?;
        let envelope: SoapEnvelope = quick_xml::de::from_str(&xml)?;
        let msgr_config: MsgrConfig = quick_xml::de::from_str(
            &envelope
                .soap_body
                .get_client_config_response
                .get_client_config_result,
        )?;

        Ok(Config {
            tabs: msgr_config.tab_config.msn_tab_data.tab,
            msn_today_url: msgr_config.localized_config.msn_today_config.msn_today_url,
        })
    }
}

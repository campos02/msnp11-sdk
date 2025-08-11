use crate::errors::sdk_error::SdkError;
use reqwest::header::{AUTHORIZATION, HeaderMap};
use std::error::Error;

pub struct PassportAuth<'a> {
    client: reqwest::Client,
    nexus_url: &'a str,
}

impl<'a> PassportAuth<'a> {
    pub fn new(nexus_url: &'a str) -> Self {
        Self {
            client: reqwest::Client::new(),
            nexus_url,
        }
    }

    async fn get_login_srf(&self) -> Result<String, Box<dyn Error>> {
        let response = self.client.get(self.nexus_url).send().await?;
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
        authorization_string: &str,
    ) -> Result<String, SdkError> {
        let login_srf = self
            .get_login_srf()
            .await
            .or(Err(SdkError::CouldNotGetAuthenticationString))?;

        let mut headers = HeaderMap::new();
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
}

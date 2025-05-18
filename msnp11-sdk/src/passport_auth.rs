use crate::msnp_error::MsnpError;
use reqwest::header::{AUTHORIZATION, HeaderMap};
use std::error::Error;

pub struct PassportAuth {
    client: reqwest::Client,
    nexus_url: String,
}

impl PassportAuth {
    pub fn new(nexus_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            nexus_url,
        }
    }

    async fn get_login_srf(&self) -> Result<String, Box<dyn Error>> {
        let response = self.client.get(&self.nexus_url).send().await?;
        let mut url = response
            .headers()
            .get("Passporturls")
            .unwrap()
            .to_str()?
            .replace("DALogin=", "");

        if !url.starts_with("http") {
            url.insert_str(0, "https://");
        }

        Ok(url)
    }

    pub async fn get_passport_token(
        &self,
        email: String,
        password: String,
        authorization_string: String,
    ) -> Result<String, Box<dyn Error>> {
        let login_srf = self.get_login_srf().await?;

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Passport1.4 OrgVerb=GET,OrgURL=http%3A%2F%2Fmessenger%2Emsn%2Ecom,sign-in={email},pwd={password},{authorization_string}").parse()?);

        let response = self.client.get(login_srf).headers(headers).send().await?;
        let authentication_info = response
            .headers()
            .get("Authentication-Info")
            .ok_or_else(|| MsnpError::AuthenticationHeaderNotFound)?
            .to_str()?;

        let token = authentication_info
            .split("from-PP='")
            .collect::<Vec<&str>>()[1];

        Ok(token.replace("'", ""))
    }
}

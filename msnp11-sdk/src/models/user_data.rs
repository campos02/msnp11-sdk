#[derive(Debug, Clone)]
pub(crate) struct UserData {
    pub(crate) email: Option<String>,
    pub(crate) display_picture: Option<Vec<u8>>,
    pub(crate) msn_object: Option<String>,
}

impl UserData {
    pub(crate) fn new() -> Self {
        Self {
            email: None,
            display_picture: None,
            msn_object: None,
        }
    }
}

/// MSNP presence statuses.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum MsnpStatus {
    Online,
    Busy,
    Away,
    Idle,
    OutToLunch,
    OnThePhone,
    BeRightBack,
    AppearOffline,
}

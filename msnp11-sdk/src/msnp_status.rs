/// MSNP presence statuses.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
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

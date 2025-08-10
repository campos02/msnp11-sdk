/// MSNP lists.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum MsnpList {
    /// Your contact list as it appears in the client.
    ForwardList,

    /// List of people allowed to talk to you and see your presence (only applies if using a [BLP][crate::client::Client::set_blp] of `BL`).
    AllowList,

    /// What contacts are blocked.
    BlockList,

    ReverseList,
    PendingList,
}

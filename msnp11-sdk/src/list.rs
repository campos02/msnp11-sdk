#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum List {
    ForwardList,
    AllowList,
    BlockList,
    ReverseList,
    PendingList,
}

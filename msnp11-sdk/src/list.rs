#[derive(Debug, Clone, PartialEq)]
pub enum List {
    ForwardList,
    AllowList,
    BlockList,
    ReverseList,
    PendingList,
}

pub struct RpcArg {
    data: &'static str,
}

impl RpcArg {
    pub fn example_arg() -> Self {
        RpcArg {
            data: "Example rpc arg",
        }
    }
}

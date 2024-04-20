// Local
use spectre_runtime::opaque::Block;

/// A set of APIs that ethereum-compatible runtimes must implement.
pub trait EthCompatRuntimeApiCollection:
    sp_api::ApiExt<Block>
    + fp_rpc::EthereumRuntimeRPCApi<Block>
    + fp_rpc::ConvertTransactionRuntimeApi<Block>
{
}

impl<Api> EthCompatRuntimeApiCollection for Api where
    Api: sp_api::ApiExt<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + fp_rpc::ConvertTransactionRuntimeApi<Block>
{
}

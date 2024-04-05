//! Run with
//!
//! ```not_rust
//! cargo run -p beacon-api-sidecar-fetcher -- node

use std::{
    collections::VecDeque,
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures_util::{stream::FuturesUnordered, Future, FutureExt, Stream, StreamExt};
use reqwest::Error;
use reth::{providers::CanonStateNotification, transaction_pool::TransactionPoolExt};

use serde::{self, Deserialize, Serialize};
use tracing::debug;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    Ok(())
}

//TODO look at PeersManager.
//TODO Figure out pending_requests/queued_actions
//Add Reqwest logic
//Create custom tests.
#[derive(Debug)]
pub struct MinedSidecarStream<St, P>
where
    St: Stream<Item = CanonStateNotification> + Send + Unpin + 'static,
{
    events: St,
    pool: P,
    client: reqwest::Client,
    pending_requests:
        FuturesUnordered<Pin<Box<dyn Future<Output = Result<BlobSidecar, reqwest::Error>> + Send>>>, /* will contant CL queries. */
    queued_actions: VecDeque<BlobSidecar>, // Buffer for ready items
}

impl<St, P> Stream for MinedSidecarStream<St, P>
where
    St: Stream<Item = CanonStateNotification> + Send + Unpin + 'static,
    P: TransactionPoolExt + Unpin + 'static,
{
    type Item = Result<BlobSidecar, reqwest::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        // return any buffered result
        if let Some(blob_sidecar) = this.queued_actions.pop_front() {
            return Poll::Ready(Some(Ok(blob_sidecar)));
        }

        // Check if any pending reqwests are ready.
        while let Poll::Ready(Some(result)) = this.pending_requests.poll_next_unpin(cx) {
            match result {
                Ok(blob_sidecar) => return Poll::Ready(Some(Ok(blob_sidecar))),
                Err(e) => {
                    debug!(error = %e, "Error processing a pending consensus layer request.");
                }
            }
        }

        // TODO: Add fetching logic here.
        loop {
            match this.events.poll_next_unpin(cx) {
                Poll::Ready(Some(notification)) => {
                    // Logic goes here to one check if pool exists else query CL\
                    // Pool logic added to queued actions?
                    // CL Query request added to pending_requests
                    //Box::pin(async move { request })
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => continue,
            }
        }
    }
}
///TODO Add
impl<St, P> MinedSidecarStream<St, P>
where
    St: Stream<Item = CanonStateNotification> + Send + Unpin + 'static,
    P: TransactionPoolExt + Unpin + 'static,
{
    // Ensure this method transforms a CanonStateNotification into a BlobSidecar
    fn data_exists(&mut self, item: &CanonStateNotification) -> BlobSidecar {
        // Transformation logic here
        // For demonstration, let's return a default BlobSidecar for now
        BlobSidecar { ..Default::default() }
    }
}
/// TODO: Import as feature
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlobSidecar {
    pub data: Vec<Data>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub index: String,
    pub blob: String,
    #[serde(rename = "kzg_commitment")]
    pub kzg_commitment: String,
    #[serde(rename = "kzg_proof")]
    pub kzg_proof: String,
    #[serde(rename = "signed_block_header")]
    pub signed_block_header: SignedBlockHeader,
    #[serde(rename = "kzg_commitment_inclusion_proof")]
    pub kzg_commitment_inclusion_proof: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedBlockHeader {
    pub message: Message,
    pub signature: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub slot: String,
    #[serde(rename = "proposer_index")]
    pub proposer_index: String,
    #[serde(rename = "parent_root")]
    pub parent_root: String,
    #[serde(rename = "state_root")]
    pub state_root: String,
    #[serde(rename = "body_root")]
    pub body_root: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlobError {
    #[serde(rename = "code")]
    code: u16,
    #[serde(rename = "message")]
    message: String,
}

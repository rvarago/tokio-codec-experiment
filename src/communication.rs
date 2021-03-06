use super::codec;
use super::store;
use anyhow::Result;
use futures::{SinkExt, StreamExt};
use log::info;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

pub struct StoreProtocol<T> {
    framed: Framed<T, codec::LineQueryCodec>,
    store_tx: store::Sender,
}

impl<T> StoreProtocol<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(conn: T, store_tx: store::Sender) -> Self {
        Self {
            framed: Framed::new(conn, codec::LineQueryCodec::new()),
            store_tx,
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        while let Some(req) = self.framed.next().await {
            let res = self.process(req?).await?;
            self.framed.send(res).await?;
        }
        Ok(())
    }

    async fn process(&mut self, req: codec::Request) -> Result<codec::Response> {
        match req {
            codec::Request::Get { key } => {
                info!("Get: key: {}", key);
                let value = self.get_from_store(&key).await?;
                Ok(codec::Response::Get { key, value })
            }
            codec::Request::Set { key, value } => {
                info!("Set: key: {} value: {}", key, value);
                self.set_into_store(&key, value).await?;
                Ok(codec::Response::Set { key })
            }
        }
    }

    async fn get_from_store(&mut self, key: &String) -> Result<Option<String>> {
        store::get(key, &mut self.store_tx).await
    }

    async fn set_into_store(&mut self, key: &String, value: String) -> Result<()> {
        store::set(key, value, &mut self.store_tx).await
    }
}

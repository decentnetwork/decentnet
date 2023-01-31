use std::io;

use async_trait::async_trait;
use libp2p::{
    core::{
        upgrade::{read_length_prefixed, write_length_prefixed},
        ProtocolName,
    },
    futures::{AsyncRead, AsyncWrite, FutureExt},
    request_response::RequestResponseCodec,
};
use rkyv::{
    archived_root,
    ser::{serializers::AllocSerializer, Serializer},
    Archive, Deserialize, Infallible, Serialize,
};

use super::NetworkNode;

#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub struct NetworkNodeRecord {
    pub nodes: Vec<NetworkNode>,
}

#[derive(Clone)]
pub struct DecentNetProtocol();

#[derive(Clone)]
pub struct DecentNetCodec();

#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub enum DecentNetRequest {
    Ping,
    GetNetworkNodes,
    SendNodeRecord(NetworkNodeRecord),
}

impl From<Vec<u8>> for DecentNetRequest {
    fn from(bytes: Vec<u8>) -> Self {
        let archived = unsafe { archived_root::<DecentNetRequest>(&bytes[..]) };
        let req = archived.deserialize(&mut Infallible);
        // let req =
        req.expect("Deserilization Failed")
        // match req {
        //     DecentNetRequest::Ping => DecentNetRequest::Ping,
        //     DecentNetRequest::GetNetworkNodes => DecentNetRequest::GetNetworkNodes,
        //     DecentNetRequest::SendNodeRecord(record) => DecentNetRequest::SendNodeRecord(record),
        // }
        // req
    }
}

impl From<DecentNetRequest> for Vec<u8> {
    fn from(request: DecentNetRequest) -> Self {
        let mut serializer = AllocSerializer::<256>::default();
        serializer.serialize_value(&request).unwrap();
        serializer.into_serializer().into_inner().to_vec()
    }
}

#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub enum DecentNetResponse {
    Pong,
    Record(NetworkNodeRecord),
    GotNetworkRecord,
}

impl From<Vec<u8>> for DecentNetResponse {
    fn from(bytes: Vec<u8>) -> Self {
        let archived = unsafe { archived_root::<DecentNetResponse>(&bytes[..]) };
        let res = archived.deserialize(&mut Infallible);
        res.expect("deserialization failed")
        // match res {
        //     DecentNetResponse::Pong => DecentNetResponse::Pong,
        //     DecentNetResponse::Record(NetworkNodeRecord { nodes }) => {
        //         DecentNetResponse::Record(NetworkNodeRecord { nodes })
        //     }
        // }
    }
}

impl From<DecentNetResponse> for Vec<u8> {
    fn from(res: DecentNetResponse) -> Self {
        let mut serializer = AllocSerializer::<256>::default();
        serializer.serialize_value(&res).unwrap();
        serializer.into_serializer().into_inner().to_vec()
    }
}

impl ProtocolName for DecentNetProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/decentnet/0.0.1"
    }
}

#[async_trait]
impl RequestResponseCodec for DecentNetCodec {
    type Protocol = DecentNetProtocol;
    type Request = DecentNetRequest;
    type Response = DecentNetResponse;

    async fn read_request<T>(
        &mut self,
        _: &DecentNetProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_length_prefixed(io, 2048)
            .map(|request| match request {
                Ok(bytes) => Ok(DecentNetRequest::from(bytes)),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
            })
            .await
    }

    async fn read_response<T>(
        &mut self,
        _: &DecentNetProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        //TODO? : Check for max_size bugs here.
        read_length_prefixed(io, 2048)
            .map(|response| match response {
                Ok(bytes) => Ok(DecentNetResponse::from(bytes)),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
            })
            .await
    }

    async fn write_request<T>(
        &mut self,
        _: &DecentNetProtocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let bytes = Vec::from(req);
        write_length_prefixed(io, &bytes).await
    }

    async fn write_response<T>(
        &mut self,
        _: &DecentNetProtocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buf = Vec::from(res);
        write_length_prefixed(io, buf).await
    }
}

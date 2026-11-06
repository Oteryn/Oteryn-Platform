//! Prost field declarations for pinned foundation.proto (no replacement envelope).
use prost::Message;
#[derive(Clone, PartialEq, Message)]
pub struct TransactionEventRef {
    #[prost(bytes = "vec", tag = "1")]
    pub transaction_id: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub ordinal: u32,
    #[prost(uint32, tag = "3")]
    pub count: u32,
}
#[derive(Clone, PartialEq, Message)]
pub struct CommandRef {
    #[prost(bytes = "vec", tag = "1")]
    pub game_session_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub command_id: u64,
}
#[derive(Clone, PartialEq, Message)]
pub struct AnalyticsActorRef {
    #[prost(string, tag = "1")]
    pub identity_domain: String,
    #[prost(uint64, tag = "2")]
    pub identity_epoch: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub analytics_actor_id: Vec<u8>,
}
#[derive(Clone, PartialEq, Message)]
pub struct RuntimeOrderRef {
    #[prost(uint64, tag = "1")]
    pub scope_ownership_generation: u64,
    #[prost(uint64, tag = "2")]
    pub runtime_execution_ordinal: u64,
}
#[derive(Clone, PartialEq, Message)]
pub struct CausationRef {
    #[prost(oneof = "Cause", tags = "1,2,3,4")]
    pub cause: Option<Cause>,
}
#[derive(Clone, PartialEq, prost::Oneof)]
pub enum Cause {
    #[prost(bytes, tag = "1")]
    EventId(Vec<u8>),
    #[prost(message, tag = "2")]
    Command(CommandRef),
    #[prost(bytes, tag = "3")]
    OperationId(Vec<u8>),
    #[prost(bytes, tag = "4")]
    TransactionId(Vec<u8>),
}
#[derive(Clone, PartialEq, Message)]
pub struct EventEnvelope {
    #[prost(uint32, tag = "1")]
    pub envelope_revision: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub event_id: Vec<u8>,
    #[prost(uint32, tag = "3")]
    pub event_type_id: u32,
    #[prost(uint32, tag = "4")]
    pub event_schema_revision: u32,
    #[prost(int32, tag = "5")]
    pub durability_class: i32,
    #[prost(int32, tag = "6")]
    pub privacy_class: i32,
    #[prost(string, tag = "7")]
    pub retention_profile_id: String,
    #[prost(int64, tag = "8")]
    pub occurred_at_unix_ms: i64,
    #[prost(bytes = "vec", optional, tag = "9")]
    pub world_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "10")]
    pub channel_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "11")]
    pub instance_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "12")]
    pub node_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "13")]
    pub game_session_id: Option<Vec<u8>>,
    #[prost(uint64, optional, tag = "14")]
    pub connection_generation: Option<u64>,
    #[prost(message, optional, tag = "15")]
    pub runtime_order: Option<RuntimeOrderRef>,
    #[prost(uint64, optional, tag = "16")]
    pub command_id: Option<u64>,
    #[prost(bytes = "vec", optional, tag = "17")]
    pub operation_id: Option<Vec<u8>>,
    #[prost(message, optional, tag = "18")]
    pub transaction_event: Option<TransactionEventRef>,
    #[prost(bytes = "vec", optional, tag = "19")]
    pub correlation_id: Option<Vec<u8>>,
    #[prost(message, optional, tag = "20")]
    pub causation: Option<CausationRef>,
    #[prost(message, optional, tag = "21")]
    pub analytics_actor: Option<AnalyticsActorRef>,
    #[prost(uint32, optional, tag = "22")]
    pub protocol_major: Option<u32>,
    #[prost(string, optional, tag = "23")]
    pub ruleset_revision: Option<String>,
    #[prost(string, optional, tag = "24")]
    pub content_revision: Option<String>,
    #[prost(string, tag = "25")]
    pub server_build_id: String,
    #[prost(bytes = "vec", tag = "26")]
    pub payload: Vec<u8>,
    #[prost(bytes = "vec", tag = "27")]
    pub payload_sha256: Vec<u8>,
}

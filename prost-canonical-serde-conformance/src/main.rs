//! Conformance test harness for prost canonical JSON support.
//!
//! Run this binary under the protobuf `conformance_test_runner` to exercise
//! JSON and binary conformance cases against this crate's implementation.
extern crate alloc;

use std::io::{self, Read, Write};

use prost::Message;
use prost_canonical_serde::{Canonical, CanonicalDeserialize, CanonicalSerialize, CanonicalValue};

pub mod conformance {
    #![expect(
        clippy::doc_markdown,
        reason = "Generated protobuf code uses upstream docs ."
    )]
    include!(concat!(env!("OUT_DIR"), "/conformance.rs"));
}

pub mod conformance_proto2 {
    #![expect(
        clippy::doc_markdown,
        reason = "Generated protobuf code uses upstream docs ."
    )]
    include!(concat!(
        env!("OUT_DIR"),
        "/protobuf_test_messages.proto2.rs"
    ));
}

pub mod conformance_proto3 {
    #![expect(
        clippy::doc_markdown,
        reason = "Generated protobuf code uses upstream docs ."
    )]
    include!(concat!(
        env!("OUT_DIR"),
        "/protobuf_test_messages.proto3.rs"
    ));
}

use conformance::conformance_request::Payload;
use conformance::conformance_response::Result as ResponseResult;
use conformance::{ConformanceRequest, ConformanceResponse, FailureSet, TestCategory, WireFormat};
use conformance_proto2::TestAllTypesProto2;
use conformance_proto3::TestAllTypesProto3;

fn read_frame() -> io::Result<Option<Vec<u8>>> {
    let mut len_bytes = [0u8; 4];
    match io::stdin().read_exact(&mut len_bytes) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err),
    }
    let len = u32::from_le_bytes(len_bytes) as usize;
    let mut buf = vec![0u8; len];
    io::stdin().read_exact(&mut buf)?;
    Ok(Some(buf))
}

fn write_frame(response: &ConformanceResponse) -> io::Result<()> {
    let mut buf = Vec::new();
    response.encode(&mut buf).expect("encode response");
    let len = u32::try_from(buf.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "response frame too large"))?;
    let mut stdout = io::stdout();
    stdout.write_all(&len.to_le_bytes())?;
    stdout.write_all(&buf)?;
    stdout.flush()
}

fn parse_json<T: CanonicalDeserialize>(input: &str) -> Result<T, String> {
    serde_json::from_str::<CanonicalValue<T>>(input)
        .map(|value| value.0)
        .map_err(|err| err.to_string())
}

fn to_json<T: CanonicalSerialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(&Canonical::new(value)).map_err(|err| err.to_string())
}

fn decode_proto<T: Message + Default>(bytes: &[u8]) -> Result<T, String> {
    T::decode(bytes).map_err(|err| err.to_string())
}

fn encode_proto<T: Message>(value: &T) -> Vec<u8> {
    value.encode_to_vec()
}

fn skipped(message: &str) -> ConformanceResponse {
    ConformanceResponse {
        result: Some(ResponseResult::Skipped(message.to_string())),
    }
}

fn parse_error(message: &str) -> ConformanceResponse {
    ConformanceResponse {
        result: Some(ResponseResult::ParseError(message.to_string())),
    }
}

fn serialize_error(message: &str) -> ConformanceResponse {
    ConformanceResponse {
        result: Some(ResponseResult::SerializeError(message.to_string())),
    }
}

fn protobuf_response(payload: Vec<u8>) -> ConformanceResponse {
    ConformanceResponse {
        result: Some(ResponseResult::ProtobufPayload(payload)),
    }
}

fn json_response(payload: String) -> ConformanceResponse {
    ConformanceResponse {
        result: Some(ResponseResult::JsonPayload(payload)),
    }
}

fn failure_list_response() -> ConformanceResponse {
    let failures = FailureSet::default();
    let payload = failures.encode_to_vec();
    protobuf_response(payload)
}

fn handle_message<T>(
    input_payload: &Payload,
    output_format: WireFormat,
    _test_category: TestCategory,
) -> ConformanceResponse
where
    T: Message + Default + CanonicalDeserialize + CanonicalSerialize,
{
    let message = match input_payload {
        Payload::ProtobufPayload(bytes) => match decode_proto::<T>(bytes) {
            Ok(value) => value,
            Err(err) => return parse_error(&err),
        },
        Payload::JsonPayload(json) => match parse_json::<T>(json) {
            Ok(value) => value,
            Err(err) => return parse_error(&err),
        },
        Payload::TextPayload(_) => {
            return skipped("text format input not supported");
        }
        Payload::JspbPayload(_) => {
            return skipped("jspb input not supported");
        }
    };

    match output_format {
        WireFormat::Protobuf => protobuf_response(encode_proto(&message)),
        WireFormat::Json => match to_json(&message) {
            Ok(json) => json_response(json),
            Err(err) => serialize_error(&err),
        },
        WireFormat::TextFormat => skipped("text format output not supported"),
        WireFormat::Jspb => skipped("jspb output not supported"),
        WireFormat::Unspecified => skipped("unspecified output format"),
    }
}

fn handle_request(request: ConformanceRequest) -> ConformanceResponse {
    if request.message_type == "conformance.FailureSet" {
        return failure_list_response();
    }

    let output_format =
        WireFormat::try_from(request.requested_output_format).unwrap_or(WireFormat::Unspecified);
    let test_category =
        TestCategory::try_from(request.test_category).unwrap_or(TestCategory::UnspecifiedTest);

    let Some(payload) = request.payload else {
        return skipped("no payload provided");
    };

    match request.message_type.as_str() {
        "protobuf_test_messages.proto2.TestAllTypesProto2" => {
            handle_message::<TestAllTypesProto2>(&payload, output_format, test_category)
        }
        "protobuf_test_messages.proto3.TestAllTypesProto3" => {
            handle_message::<TestAllTypesProto3>(&payload, output_format, test_category)
        }
        _ => skipped("unsupported message type"),
    }
}

fn main() -> io::Result<()> {
    while let Some(frame) = read_frame()? {
        let request = match ConformanceRequest::decode(&frame[..]) {
            Ok(request) => request,
            Err(err) => {
                let response = parse_error(&err.to_string());
                write_frame(&response)?;
                continue;
            }
        };
        let response = handle_request(request);
        write_frame(&response)?;
    }
    Ok(())
}

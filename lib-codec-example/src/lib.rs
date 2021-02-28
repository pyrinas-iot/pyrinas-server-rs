#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_builtins)]

#[cfg(target_arch = "arm")]
extern crate panic_halt;

use serde::{Deserialize, Serialize};
use serde_cbor::ser::SliceWrite;
use serde_cbor::Serializer;

/// Respnse of the encode function.
/// Includes the raw bytes that can then be
/// sent, shipped, packed, zipped, whatever your hearts content.
#[repr(C)]
pub struct Encoded {
    data: [u8; 96],
    size: usize,
    resp: CodecResponse,
}

/// Struct that handles error codes
#[repr(C)]
pub enum CodecResponse {
    Ok = 0,
    EncodeError = -1,
    DecodeError = -2,
}

/// Data that is being encoded/decoded
#[repr(C)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvironmentData {
    /// temperature is represented 1000x its actual value. No floats
    temperature: u16,
    /// humidity is only indicated at 1000x its value as well. No floats.
    humidity: u16,
}

/// Encode environment data generated by a temeprature/humidity sensor
/// being read by the nRF9160 Feather.
///
/// Data is presented to the function as a struct with the corresponding
/// temperature and humidty entries.
///
/// Returns an Encoded struct which includes an output buffer, the amount of
/// data written and the CodecResponse. An error is returned when encoding or
/// decoding cannot happen.
#[no_mangle]
pub extern "C" fn encode_environment_data(data: &EnvironmentData) -> Encoded {
    // Encode
    let mut encoded = Encoded {
        data: [0; 96],
        size: 0,
        resp: CodecResponse::Ok,
    };

    // Create the writer
    let writer = SliceWrite::new(&mut encoded.data);

    // Creating Serializer with the "packed" format option. Saving critical bytes!
    let mut ser = Serializer::new(writer).packed_format();

    // Encode the data
    match data.serialize(&mut ser) {
        Ok(_) => {
            // Get the number of bytes written..
            let writer = ser.into_inner();
            encoded.size = writer.bytes_written();
        }
        Err(_) => encoded.resp = CodecResponse::EncodeError,
    };

    // Return the encoded data
    encoded
}

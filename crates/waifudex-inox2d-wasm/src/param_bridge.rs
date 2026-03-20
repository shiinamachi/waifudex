use std::io::{Cursor, Read};

use serde::Serialize;
use serde_json::Value;

const INP_MAGIC: &[u8; 8] = b"TRNSRTS\0";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PuppetParam {
    pub name: String,
    pub is_vec2: bool,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub defaults: [f32; 2],
}

#[derive(Debug, thiserror::Error)]
pub enum ParamBridgeError {
    #[error("failed to read INP payload: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid INP magic header")]
    InvalidMagic,
    #[error("failed to parse INP JSON payload: {0}")]
    Json(#[from] serde_json::Error),
    #[error("parameter entry {index} is missing a valid `{field}` field")]
    InvalidParamField { index: usize, field: &'static str },
}

pub fn extract_available_params(inp_bytes: &[u8]) -> Result<Vec<PuppetParam>, ParamBridgeError> {
    let payload = extract_payload(inp_bytes)?;
    let root: Value = serde_json::from_slice(&payload)?;
    let Some(params) = root.get("param").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    params
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let Some(param) = value.as_object() else {
                return Err(ParamBridgeError::InvalidParamField {
                    index,
                    field: "param",
                });
            };

            Ok(PuppetParam {
                name: param
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .ok_or(ParamBridgeError::InvalidParamField {
                        index,
                        field: "name",
                    })?,
                is_vec2: param
                    .get("is_vec2")
                    .and_then(Value::as_bool)
                    .ok_or(ParamBridgeError::InvalidParamField {
                        index,
                        field: "is_vec2",
                    })?,
                min: read_vec2(param.get("min"), index, "min")?,
                max: read_vec2(param.get("max"), index, "max")?,
                defaults: read_vec2(param.get("defaults"), index, "defaults")?,
            })
        })
        .collect()
}

fn extract_payload(inp_bytes: &[u8]) -> Result<Vec<u8>, ParamBridgeError> {
    let mut cursor = Cursor::new(inp_bytes);
    let mut magic = [0_u8; 8];
    cursor.read_exact(&mut magic)?;
    if &magic != INP_MAGIC {
        return Err(ParamBridgeError::InvalidMagic);
    }

    let mut length_buf = [0_u8; 4];
    cursor.read_exact(&mut length_buf)?;
    let payload_len = u32::from_be_bytes(length_buf) as usize;
    let mut payload = vec![0_u8; payload_len];
    cursor.read_exact(&mut payload)?;
    Ok(payload)
}

fn read_vec2(value: Option<&Value>, index: usize, field: &'static str) -> Result<[f32; 2], ParamBridgeError> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Err(ParamBridgeError::InvalidParamField { index, field });
    };

    if values.len() != 2 {
        return Err(ParamBridgeError::InvalidParamField { index, field });
    }

    let x = values[0]
        .as_f64()
        .ok_or(ParamBridgeError::InvalidParamField { index, field })? as f32;
    let y = values[1]
        .as_f64()
        .ok_or(ParamBridgeError::InvalidParamField { index, field })? as f32;
    Ok([x, y])
}


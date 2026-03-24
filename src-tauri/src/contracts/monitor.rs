use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DisplayMonitorOption {
    pub id: String,
    pub label: String,
    pub work_area_left: i32,
    pub work_area_top: i32,
    pub work_area_width: u32,
    pub work_area_height: u32,
}

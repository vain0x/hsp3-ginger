//! HSP から提供されるデバッグ情報 (DINFO) を解析する。
//! hspsdk/hsp3code.txt を参照。

use crate::helpers;
use crate::hspsdk;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::{cmp, slice};

/// 埋め込まれているデータ (文字列や浮動小数点数) の識別子。
/// Data Segment のオフセット (DSオフセット値) に等しい。
pub(crate) type DataId = usize;

/// ラベルの識別子。OTオフセット値。
pub(crate) type LabelId = usize;

/// 構造体やパラメーターの識別子。STオフセット値。
pub(crate) type StructId = usize;

pub(crate) trait ConstantMap: Clone {
    fn get_string(&self, id: DataId) -> String;
    fn get_float(&self, id: DataId) -> f64;
}

#[derive(Clone, Debug)]
pub(crate) struct HspConstantMap {
    pub data_segment: Arc<Vec<u8>>,
}

impl ConstantMap for HspConstantMap {
    fn get_string(&self, id: DataId) -> String {
        let p = self.data_segment[id..].as_ptr() as *const u8;
        helpers::string_from_hsp_str(p)
    }

    fn get_float(&self, id: DataId) -> f64 {
        unsafe { *(self.data_segment[id..].as_ptr() as *const f64) }
    }
}

/// DINFO の文脈。データに書かれている名前が何の名前であるかは、この文脈によって決まる。
#[derive(PartialEq, Clone, Copy, Debug)]
enum Mode {
    /// ソースコードが現れる文脈
    SourceLocations,
    /// ラベルの名前が識別子として現れる文脈
    Labels,
    /// パラメーターの名前が識別子として現れる文脈
    Params,
}

impl Mode {
    fn next(&self) -> Self {
        match self {
            Mode::SourceLocations => Mode::Labels,
            Mode::Labels => Mode::Params,
            Mode::Params => Mode::Params,
        }
    }
}

struct DebugSegmentParser<'a, C: ConstantMap> {
    debug_segment: &'a [u8],
    /// `debug_segment` のいま見ている位置
    di: usize,
    code_segment: &'a [u8],
    /// `code_segment` のいま見ている位置
    ci: usize,
    mode: Mode,
    current_file_name: Option<DataId>,
    current_line: i32,
    file_names: Vec<DataId>,
    label_names: BTreeMap<LabelId, DataId>,
    param_names: BTreeMap<StructId, DataId>,
    constant_map: Arc<C>,
}

impl<'a> DebugSegmentParser<'a, HspConstantMap> {
    pub fn from_hspctx(hspctx: &hspsdk::HSPCTX) -> Self {
        let debug_segment_size = unsafe { *hspctx.hsphed }.max_dinfo;
        let debug_segment = from_segment(hspctx.mem_di as *const u8, debug_segment_size);

        let code_segment_size = unsafe { *hspctx.hsphed }.max_cs;
        let code_segment = from_segment(hspctx.mem_mcs as *const u8, code_segment_size);

        let data_segment_size = unsafe { *hspctx.hsphed }.max_ds;
        let data_segment = from_segment(hspctx.mem_mds as *const u8, data_segment_size);
        let data_segment = Arc::new(data_segment.to_owned());
        let constant_map = Arc::new(HspConstantMap { data_segment });

        DebugSegmentParser::new(debug_segment, code_segment, constant_map)
    }
}

impl<'a, C: ConstantMap> DebugSegmentParser<'a, C> {
    fn new(debug_segment: &'a [u8], code_segment: &'a [u8], constant_map: Arc<C>) -> Self {
        DebugSegmentParser {
            debug_segment,
            di: 0,
            code_segment,
            ci: 0,
            mode: Mode::SourceLocations,
            current_file_name: None,
            current_line: 0,
            file_names: Vec::new(),
            label_names: BTreeMap::new(),
            param_names: BTreeMap::new(),
            constant_map,
        }
    }

    fn on_corruption_detected(&self) -> ! {
        panic!("DINFO が破損しています。")
    }

    fn read_1byte(&mut self) -> u8 {
        if self.di >= self.debug_segment.len() {
            self.on_corruption_detected()
        }

        let value = self.debug_segment[self.di];
        self.di += 1;
        value
    }

    fn read_2byte(&mut self) -> u16 {
        if self.di + 1 >= self.debug_segment.len() {
            self.on_corruption_detected()
        }

        let value = wpeek(&self.debug_segment[self.di..]);
        self.di += 2;
        value
    }

    fn read_3byte(&mut self) -> u32 {
        if self.di >= self.debug_segment.len() {
            self.on_corruption_detected()
        }

        let value = tpeek(&self.debug_segment[self.di..]);
        self.di += 2;
        value
    }

    pub fn parse(mut self) -> DebugInfo<C> {
        while self.di < self.debug_segment.len() {
            match self.read_1byte() {
                0xFF => self.on_mode_switch(),
                0xFE => self.on_source_location(),
                0xFD | 0xFB => self.on_name(),
                0xFC => self.on_next_line2(),
                offset => self.on_next_line1(offset as usize),
            }
        }

        DebugInfo {
            file_names: self.file_names,
            label_names: self.label_names,
            param_names: self.param_names,
            constant_map: self.constant_map,
        }
    }

    /// 現在のモードの終端に達したとき。続きは次のモードで解釈する。
    fn on_mode_switch(&mut self) {
        self.mode = self.mode.next();
    }

    /// ソース位置の情報が出現したとき。
    fn on_source_location(&mut self) {
        let file_name_id = self.read_3byte();
        let line = self.read_2byte();

        self.current_file_name = if file_name_id != 0 {
            Some(file_name_id as DataId)
        } else {
            None
        };
        self.current_line = line as i32;

        if file_name_id != 0 {
            self.file_names.push(file_name_id as DataId);
        }
    }

    /// 名前情報が出現したとき。
    fn on_name(&mut self) {
        let name_id = self.read_3byte();
        let some_id = self.read_2byte();

        let names = match self.mode {
            Mode::Labels => &mut self.label_names,
            Mode::Params => &mut self.param_names,
            Mode::SourceLocations => return,
        };

        names.insert(some_id as usize, name_id as DataId);
    }

    /// コード位置情報が出現したとき。 (offset が2バイトのケース)
    fn on_next_line2(&mut self) {
        let offset = self.read_2byte();

        self.ci += offset as usize;
        self.add_location_info();
    }

    /// コード位置情報が出現したとき。 (offset が1バイトのケース)
    fn on_next_line1(&mut self, offset: usize) {
        self.ci += offset;
        self.add_location_info();
    }

    fn add_location_info(&mut self) {
        // FIXME: いまのところ必要ないので省略
        // ここで Code Segment の位置とファイル名・行番号の対応を記録することで、
        // 実行時のコード位置からファイル位置を復元できるようになる
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DebugInfo<C: ConstantMap> {
    file_names: Vec<DataId>,
    label_names: BTreeMap<LabelId, DataId>,
    param_names: BTreeMap<StructId, DataId>,
    constant_map: Arc<C>,
}

impl DebugInfo<HspConstantMap> {
    pub fn parse_hspctx(hspctx: &hspsdk::HSPCTX) -> Self {
        DebugSegmentParser::from_hspctx(hspctx).parse()
    }
}

impl<C: ConstantMap> DebugInfo<C> {
    pub fn parse(debug_segment: &[u8], code_segment: &[u8], constant_map: Arc<C>) -> Self {
        DebugSegmentParser::new(debug_segment, code_segment, constant_map).parse()
    }

    pub fn file_names(&self) -> Vec<String> {
        let mut file_names = self
            .file_names
            .iter()
            .map(|&data_id| self.constant_map.get_string(data_id))
            .collect::<Vec<_>>();
        file_names.sort();
        file_names.dedup();
        file_names
    }

    pub fn find_label_name(&self, label_id: LabelId) -> Option<String> {
        self.label_names
            .get(&label_id)
            .map(|&data_id| self.constant_map.get_string(data_id))
    }

    pub fn find_param_name(&self, struct_id: StructId) -> Option<String> {
        self.param_names
            .get(&struct_id)
            .map(|&data_id| self.constant_map.get_string(data_id))
    }
}

fn from_segment<'a>(ptr: *const u8, len: i32) -> &'a [u8] {
    let len = cmp::max(0, len) as usize;
    unsafe { slice::from_raw_parts(ptr, len) }
}

/// 2バイトを整数値として読む。
fn wpeek(data: &[u8]) -> u16 {
    data[0] as u16 | ((data[1] as u16) << 8)
}

/// 3バイトを整数値として読む。
fn tpeek(data: &[u8]) -> u32 {
    data[0] as u32 | ((data[1] as u32) << 8) | ((data[2] as u32) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use std::path::PathBuf;

    #[test]
    fn test_debug_info() {
        let mut ax = Vec::new();
        {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/hsp/deep.ax");
            let mut ax_file = fs::File::open(path).unwrap();
            ax_file.read_to_end(&mut ax).unwrap();
        }

        let hsphed = unsafe { &*(ax.as_ptr() as *const hspsdk::HSPHED) };

        let debug_segment_begin = hsphed.pt_dinfo as usize;
        let debug_segment_end = debug_segment_begin + hsphed.max_dinfo as usize;
        let debug_segment = &ax[debug_segment_begin..debug_segment_end];

        let code_segment_begin = hsphed.pt_cs as usize;
        let code_segment_end = code_segment_begin + hsphed.max_cs as usize;
        let code_segment = &ax[code_segment_begin..code_segment_end];

        let data_segment_begin = hsphed.pt_ds as usize;
        let data_segment_end = data_segment_begin + hsphed.max_ds as usize;
        let data_segment = Arc::new(ax[data_segment_begin..data_segment_end].to_owned());

        let constant_map = Arc::new(HspConstantMap { data_segment });

        let debug_info = DebugInfo::parse(debug_segment, code_segment, constant_map);
        assert_eq!(
            debug_info.file_names(),
            vec![
                "deep.hsp",
                "hspdef.as",
                "inner/inner.hsp",
                "inner_sub.hsp",
                "userdef.as",
            ]
        );
    }
}

// コマンドや配列の引数を読み込む処理

#pragma once

#include "../hspsdk/hsp3plugin.h"

// `code_getprm` の結果
enum class GetParamStatus {
	// 引数を正常に取得した。(次の字句は ')' でない。)
	Ok = PARAM_OK,

	// 引数を正常に取得した。(次の字句は ')' になる。)
	OkFinal = PARAM_SPLIT,

	// 命令の引数リストの末尾 (':' または改行) に達した。引数は取得できなかった。
	EndStmt = PARAM_END,

	// 引数が省略されていて、取得できなかった。
	Default = PARAM_DEFAULT,

	// 関数の引数リストの末尾 (')') に達した。引数は取得できなかった。
	EndFunc = PARAM_ENDSPLIT,
};

// `code_getprm` の結果として引数を取得したか？
// true なら `mpval` の値を利用できる。
inline auto param_status_is_ok(GetParamStatus status) -> bool {
	switch (status) {
	case GetParamStatus::Ok:
	case GetParamStatus::OkFinal:
		return true;

	default:
		return false;
	}
}

// flag 型のデータ ptr を target_flag 型に変換する。
// 変換できないときは、型不一致のランタイムエラーが発生することがある。
// (int() 関数などに対応する。HspVarProc::Cnv, CnvCustom が使われる。)
auto hsx_value_convert(short flag, void const* ptr, short target_flag,
                       HSPEXINFO*) -> void const*;

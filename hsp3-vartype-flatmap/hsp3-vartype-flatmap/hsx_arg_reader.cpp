#include "pch.h"

#include "hsx_arg_reader.h"

// 型不一致エラーが発生する可能性がある。
auto hsx_value_convert(short flag, void const* ptr, short target_flag,
                       HSPEXINFO* my_exinfo) -> void const* {
	// 変換先の型が組み込み型でない場合、変換先の型にしか変換方法は分からないはずなので、CnvCustom を使う。
	if (target_flag > HSPVAR_FLAG_INT) {
		auto target_proc = my_exinfo->HspFunc_getproc(target_flag);
		return target_proc->CnvCustom(ptr, flag);
	}

	auto source_proc = my_exinfo->HspFunc_getproc(flag);
	return source_proc->Cnv(ptr, target_flag);
}

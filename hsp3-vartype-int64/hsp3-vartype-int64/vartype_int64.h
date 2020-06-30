#pragma once

#include "../hspsdk/hspvar_core.h"

// int64 型の型 ID を取得する。
EXPORT auto vartype_int64_flag() -> short;

EXPORT void vartype_int64_init(HspVarProc* p);

extern auto int64_convert_from(void const* buffer, int flag) -> void*;

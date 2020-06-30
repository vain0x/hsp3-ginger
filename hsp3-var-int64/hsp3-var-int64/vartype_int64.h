#pragma once

#include "../hspsdk/hspvar_core.h"

// int64 型の型 ID を取得する。
extern int vartype_int64_id();

extern void vartype_int64_init(HspVarProc* p);

extern auto int64_convert_from(void const* buffer, int flag) -> void*;
